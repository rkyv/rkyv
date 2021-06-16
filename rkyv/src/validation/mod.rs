//! Validation implementations and helper types.

pub mod owned;
pub mod validators;

use crate::{Archive, ArchivePointee, Archived, Fallible, RelPtr};
use bytecheck::CheckBytes;
use core::{alloc::Layout, any::TypeId, fmt};
use ptr_meta::{DynMetadata, Pointee};
#[cfg(feature = "std")]
use std::error::Error;

/// Gets the layout of a type from its metadata.
pub trait LayoutMetadata<T: ?Sized> {
    /// Gets the layout of the type.
    fn layout(self) -> Layout;
}

impl<T> LayoutMetadata<T> for () {
    #[inline]
    fn layout(self) -> Layout {
        Layout::new::<T>()
    }
}

impl<T> LayoutMetadata<[T]> for usize {
    #[inline]
    fn layout(self) -> Layout {
        Layout::array::<T>(self).unwrap()
    }
}

impl LayoutMetadata<str> for usize {
    #[inline]
    fn layout(self) -> Layout {
        Layout::array::<u8>(self).unwrap()
    }
}

impl<T: ?Sized> LayoutMetadata<T> for DynMetadata<T> {
    #[inline]
    fn layout(self) -> Layout {
        self.layout()
    }
}

/// A context that can check relative pointers.
pub trait ArchiveBoundsContext: Fallible {
    /// Checks the given parts of a relative pointer for bounds issues
    ///
    /// # Safety
    ///
    /// The base pointer must be inside the archive for this context.
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error>;

    /// Checks the given memory block for bounds issues.
    ///
    /// # Safety
    ///
    /// The base pointer must be inside the archive for this context.
    unsafe fn bounds_check_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Self::Error>;
}

/// A context that can validate archive memory.
///
/// When implementing archivable containers, an archived type may point to some bytes elsewhere in
/// the archive using a [`RelPtr`]. Before checking those bytes, they must be claimed in the
/// context. This prevents infinite-loop attacks by malicious actors by ensuring that each block of
/// memory has one and only one owner.
pub trait ArchiveMemoryContext: Fallible {
    /// Claims `count` bytes located `offset` bytes away from `base`.
    ///
    /// # Safety
    ///
    /// The base pointer must be inside the archive for this context.
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error>;

    /// Claims the memory at the given location as the given type.
    ///
    /// # Safety
    ///
    /// `ptr` must be inside the archive this context was created for.
    unsafe fn claim_owned_ptr<T: ArchivePointee + ?Sized>(
        &mut self,
        ptr: *const T,
    ) -> Result<(), Self::Error>
    where
        Self: ArchiveBoundsContext,
        <T as Pointee>::Metadata: LayoutMetadata<T>,
    {
        let metadata = ptr_meta::metadata(ptr);
        let layout = LayoutMetadata::<T>::layout(metadata);
        self.bounds_check_ptr(ptr.cast(), &layout)?;
        self.claim_bytes(ptr.cast(), layout.size())?;
        Ok(())
    }

    /// Claims the memory referenced by the given relative pointer.
    fn claim_owned_rel_ptr<T: ArchivePointee + ?Sized>(
        &mut self,
        rel_ptr: &RelPtr<T>,
    ) -> Result<*const T, Self::Error>
    where
        Self: ArchiveBoundsContext,
        <T as Pointee>::Metadata: LayoutMetadata<T>,
    {
        unsafe {
            let data = self.check_rel_ptr(rel_ptr.base(), rel_ptr.offset())?;
            let ptr =
                ptr_meta::from_raw_parts::<T>(data.cast(), T::pointer_metadata(rel_ptr.metadata()));
            self.claim_owned_ptr(ptr)?;
            Ok(ptr)
        }
    }
}

/// A context that can validate shared archive memory.
///
/// Shared pointers require this kind of context to validate.
pub trait SharedArchiveContext: Fallible {
    /// Claims `count` shared bytes located `offset` bytes away from `base`.
    ///
    /// Returns whether the bytes need to be checked.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_shared_bytes(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId,
    ) -> Result<bool, Self::Error>;

    /// Claims the memory referenced by the given relative pointer.
    ///
    /// If the pointer needs to be checked, returns `Some` with the pointer to check.
    fn claim_shared_ptr<T: ArchivePointee + CheckBytes<Self> + ?Sized>(
        &mut self,
        rel_ptr: &RelPtr<T>,
        type_id: TypeId,
    ) -> Result<Option<*const T>, Self::Error>
    where
        Self: ArchiveBoundsContext,
        <T as Pointee>::Metadata: LayoutMetadata<T>,
    {
        unsafe {
            let data = self.check_rel_ptr(rel_ptr.base(), rel_ptr.offset())?;
            let metadata = T::pointer_metadata(rel_ptr.metadata());
            let ptr = ptr_meta::from_raw_parts::<T>(data.cast(), metadata);
            let layout = LayoutMetadata::<T>::layout(metadata);
            self.bounds_check_ptr(ptr.cast(), &layout)?;
            if self.claim_shared_bytes(ptr.cast(), layout.size(), type_id)? {
                Ok(Some(ptr))
            } else {
                Ok(None)
            }
        }
    }
}

/// Errors that can occur when checking an archive.
#[derive(Debug)]
pub enum CheckArchiveError<T, C> {
    /// An error that occurred while validating an object
    CheckBytesError(T),
    /// A context error occurred
    ContextError(C),
}

impl<T: fmt::Display, C: fmt::Display> fmt::Display for CheckArchiveError<T, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckArchiveError::CheckBytesError(e) => write!(f, "check bytes error: {}", e),
            CheckArchiveError::ContextError(e) => write!(f, "context error: {}", e),
        }
    }
}

#[cfg(feature = "std")]
impl<T: Error + 'static, C: Error + 'static> Error for CheckArchiveError<T, C> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CheckArchiveError::CheckBytesError(e) => Some(e as &dyn Error),
            CheckArchiveError::ContextError(e) => Some(e as &dyn Error),
        }
    }
}

/// The error type that can be produced by checking the given type with the given validator.
pub type CheckTypeError<T, C> =
    CheckArchiveError<<T as CheckBytes<C>>::Error, <C as Fallible>::Error>;

/// Checks the given archive with an additional context.
///
/// See [`check_archived_value`](crate::validation::validators::check_archived_value) for more details.
#[inline]
pub fn check_archived_value_with_context<
    'a,
    T: Archive,
    C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized,
>(
    buf: &'a [u8],
    pos: usize,
    context: &mut C,
) -> Result<&'a T::Archived, CheckTypeError<T::Archived, C>>
where
    T::Archived: CheckBytes<C> + Pointee<Metadata = ()>,
{
    unsafe {
        let data = context
            .check_rel_ptr(buf.as_ptr(), pos as isize)
            .map_err(CheckArchiveError::ContextError)?;
        let ptr = ptr_meta::from_raw_parts::<<T as Archive>::Archived>(data.cast(), ());
        let layout = LayoutMetadata::<T::Archived>::layout(());
        context
            .bounds_check_ptr(ptr.cast(), &layout)
            .map_err(CheckArchiveError::ContextError)?;
        context
            .claim_bytes(ptr.cast(), layout.size())
            .map_err(CheckArchiveError::ContextError)?;
        Ok(Archived::<T>::check_bytes(ptr, context).map_err(CheckArchiveError::CheckBytesError)?)
    }
}

/// Checks the given archive with an additional context.
///
/// See [`check_archived_value`](crate::validation::validators::check_archived_value) for more details.
#[inline]
pub fn check_archived_root_with_context<
    'a,
    T: Archive,
    C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized,
>(
    buf: &'a [u8],
    context: &mut C,
) -> Result<&'a T::Archived, CheckTypeError<T::Archived, C>>
where
    T::Archived: CheckBytes<C> + Pointee<Metadata = ()>,
{
    check_archived_value_with_context::<T, C>(
        buf,
        buf.len() - core::mem::size_of::<T::Archived>(),
        context,
    )
}
