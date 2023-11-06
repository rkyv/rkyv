//! Validation implementations and helper types.

pub mod validators;

use core::{alloc::Layout, alloc::LayoutError, any::TypeId, ops::Range};

use bytecheck::rancor::Error;
use ptr_meta::Pointee;

use crate::{Archive, ArchivePointee, CheckBytes, RelPtr};

// Replace this trait with core::mem::{align_of_val_raw, size_of_val_raw} when they get stabilized.

// TODO: Try to remove or fold into another trait
/// Gets the layout of a type from its pointee type and metadata.
pub trait LayoutRaw
where
    Self: Pointee,
{
    /// Gets the layout of the type.
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError>;
}

impl<T> LayoutRaw for T {
    #[inline]
    fn layout_raw(
        _: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        Ok(Layout::new::<T>())
    }
}

impl<T> LayoutRaw for [T] {
    #[inline]
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        Layout::array::<T>(metadata)
    }
}

impl LayoutRaw for str {
    #[inline]
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        Layout::array::<u8>(metadata)
    }
}

#[cfg(feature = "std")]
impl LayoutRaw for ::std::ffi::CStr {
    #[inline]
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        Layout::array::<::std::os::raw::c_char>(metadata)
    }
}

// TODO: Try removing depth usize and storing it on the validator instead to
// save stack space

/// A context that can validate nonlocal archive memory.
///
/// # Safety
///
/// TODO
pub unsafe trait ArchiveContext<E> {
    /// Checks that the given data address and layout is located completely within the subtree
    /// range.
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), E>;

    /// Pushes a new subtree range onto the validator and starts validating it.
    ///
    /// After calling `push_subtree_claim_to`, the validator will have a subtree range starting at
    /// the original start and ending at `root`. After popping the returned range, the validator
    /// will have a subtree range starting at `end` and ending at the original end.
    ///
    /// # Safety
    ///
    /// `root` and `end` must be located inside the archive.
    unsafe fn push_prefix_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E>;

    // TODO: try to remove suffix subtree ranges

    /// Pushes a new subtree range onto the validator and starts validating it.
    ///
    /// After calling `push_prefix_subtree_range`, the validator will have a subtree range starting
    /// at `start` and ending at `root`. After popping the returned range, the validator will have a
    /// subtree range starting at the original start and ending at `start`.
    ///
    /// # Safety
    ///
    /// `start` and `root` must be located inside the archive.
    unsafe fn push_suffix_subtree_range(
        &mut self,
        start: *const u8,
        root: *const u8,
    ) -> Result<Range<usize>, E>;

    /// Pops the given range, restoring the original state with the pushed range removed.
    ///
    /// If the range was not popped in reverse order, an error is returned.
    ///
    /// # Safety
    ///
    /// - `range` must be a range returned from this validator.
    /// - Ranges pushed after `range` must not be popped after calling
    ///   `pop_prefix_range`.
    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E>;
}

/// Helper methods for `ArchiveContext`s.
pub trait ArchiveContextExt<E>: ArchiveContext<E> {
    /// Checks that the given relative pointer to a subtree can be dereferenced.
    ///
    /// # Safety
    ///
    /// - `base` must be inside the archive this validator was created for.
    /// - `metadata` must be the metadata for the pointer defined by `base` and `offset`.
    unsafe fn bounds_check_subtree_base_offset<T: LayoutRaw + Pointee + ?Sized>(
        &mut self,
        base: *const u8,
        offset: isize,
        metadata: T::Metadata,
    ) -> Result<*const T, E>;

    /// Checks that the given `RelPtr` to a subtree can be dereferenced.
    ///
    /// # Safety
    ///
    /// - `rel_ptr` must be inside the archive this validator was created for.
    unsafe fn bounds_check_subtree_rel_ptr<T: ArchivePointee + LayoutRaw + ?Sized>(
        &mut self,
        rel_ptr: &RelPtr<T>,
    ) -> Result<*const T, E>;

    /// Pushes a new subtree range onto the validator and starts validating it.
    ///
    /// The claimed range spans from the end of `start` to the end of the current subobject range.
    ///
    /// # Safety
    ///
    /// `root` must be located inside the archive.
    unsafe fn push_prefix_subtree<T: LayoutRaw + ?Sized>(
        &mut self,
        root: *const T,
    ) -> Result<Range<usize>, E>;
}

impl<C: ArchiveContext<E> + ?Sized, E: Error> ArchiveContextExt<E> for C {
    /// Checks that the given relative pointer to a subtree can be dereferenced.
    ///
    /// # Safety
    ///
    /// - `base` must be inside the archive this validator was created for.
    /// - `metadata` must be the metadata for the pointer defined by `base` and `offset`.
    #[inline]
    unsafe fn bounds_check_subtree_base_offset<T: LayoutRaw + Pointee + ?Sized>(
        &mut self,
        base: *const u8,
        offset: isize,
        metadata: T::Metadata,
    ) -> Result<*const T, E> {
        let ptr = base.wrapping_offset(offset);
        let layout = T::layout_raw(metadata).map_err(E::new)?;
        self.check_subtree_ptr(ptr, &layout)?;
        Ok(ptr_meta::from_raw_parts(ptr.cast(), metadata))
    }

    /// Checks that the given `RelPtr` to a subtree can be dereferenced.
    ///
    /// # Safety
    ///
    /// - `rel_ptr` must be inside the archive this validator was created for.
    #[inline]
    unsafe fn bounds_check_subtree_rel_ptr<T: ArchivePointee + LayoutRaw + ?Sized>(
        &mut self,
        rel_ptr: &RelPtr<T>,
    ) -> Result<*const T, E> {
        self.bounds_check_subtree_base_offset(
            rel_ptr.base(),
            rel_ptr.offset(),
            T::pointer_metadata(rel_ptr.metadata()),
        )
    }

    /// Pushes a new subtree range onto the validator and starts validating it.
    ///
    /// The claimed range spans from the end of `start` to the end of the current subobject range.
    ///
    /// # Safety
    ///
    /// `root` must be located inside the archive.
    #[inline]
    unsafe fn push_prefix_subtree<T: LayoutRaw + ?Sized>(
        &mut self,
        root: *const T,
    ) -> Result<Range<usize>, E> {
        let layout = T::layout_raw(ptr_meta::metadata(root)).map_err(E::new)?;
        self.push_prefix_subtree_range(
            root as *const u8,
            (root as *const u8).add(layout.size()),
        )
    }
}

/// A context that can validate shared archive memory.
///
/// Shared pointers require this kind of context to validate.
pub trait SharedContext<E> {
    /// Registers the given `ptr` as a shared pointer with the given type.
    ///
    /// Returns `true` if the pointer was newly-registered and `check_bytes` should be called.
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E>;
}

/// Checks the given archive with an additional context.
///
/// See [`check_archived_value`](crate::validation::validators::check_archived_value) for more details.
#[inline]
pub fn check_archived_value_with_context<'a, T, C, E>(
    buf: &'a [u8],
    pos: isize,
    context: &mut C,
) -> Result<&'a T::Archived, E>
where
    T: Archive,
    T::Archived: CheckBytes<C, E> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Error,
{
    unsafe {
        let ptr = context.bounds_check_subtree_base_offset(buf.as_ptr(), pos, ())?;

        let range = context.push_prefix_subtree(ptr)?;
        CheckBytes::check_bytes(ptr, context)?;
        context.pop_subtree_range(range)?;

        Ok(&*ptr)
    }
}

/// Checks the given archive with an additional context.
///
/// See [`check_archived_value`](crate::validation::validators::check_archived_value) for more details.
#[inline]
pub fn check_archived_root_with_context<'a, T, C, E>(
    buf: &'a [u8],
    context: &mut C,
) -> Result<&'a T::Archived, E>
where
    T: Archive,
    T::Archived: CheckBytes<C, E> + Pointee<Metadata = ()>,
    C: ArchiveContext<E> + ?Sized,
    E: Error,
{
    check_archived_value_with_context::<T, C, E>(
        buf,
        buf.len() as isize - core::mem::size_of::<T::Archived>() as isize,
        context,
    )
}
