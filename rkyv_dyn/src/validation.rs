//! Validation implementations and helper types.

use super::{ArchivedDyn, IMPL_REGISTRY};
use bytecheck::{CheckBytes, Unreachable};
use core::{
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};
use rkyv::{
    RelPtr,
    offset_of,
    validation::{
        ArchiveBoundsContext,
        ArchiveBoundsError,
        ArchiveMemoryContext,
        ArchiveMemoryError,
        CheckBytesRef,
    },
};
use rkyv_typename::TypeName;
use std::error::Error;

pub trait DynArchiveContext {
    unsafe fn check_raw_ptr_dyn(
        &mut self,
        base: *const u8,
        offset: isize,
        len: usize,
        align: usize,
    ) -> Result<*const u8, ArchiveBoundsError>;

    unsafe fn claim_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
    ) -> Result<*const u8, ArchiveMemoryError>;
}

impl<T: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> DynArchiveContext for T {
    unsafe fn check_raw_ptr_dyn(&mut self, base: *const u8, offset: isize, len: usize, align: usize) -> Result<*const u8, ArchiveBoundsError> {
        self.check_raw_ptr(base, offset, len, align)
    }

    unsafe fn claim_bytes_dyn(&mut self, start: *const u8, len: usize) -> Result<*const u8, ArchiveMemoryError> {
        self.claim_bytes(start, len)
    }
}

impl<'a> ArchiveBoundsContext for (dyn DynArchiveContext + 'a) {
    unsafe fn check_raw_ptr(&mut self, base: *const u8, offset: isize, len: usize, align: usize) -> Result<*const u8, ArchiveBoundsError> {
        self.check_raw_ptr_dyn(base, offset, len, align)
    }
}

impl<'a> ArchiveMemoryContext for (dyn DynArchiveContext + 'a) {
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<*const u8, ArchiveMemoryError> {
        self.claim_bytes_dyn(start, len)
    }
}

// This error just always says that check bytes isn't implemented for a type
#[derive(Debug)]
struct CheckBytesUnimplemented;

impl fmt::Display for CheckBytesUnimplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "check bytes is not implemented for this type")
    }
}

impl Error for CheckBytesUnimplemented {}

type CheckBytesDyn = unsafe fn(*const u8, &mut dyn DynArchiveContext) -> Result<(), Box<dyn Error>>;

// This is the fallback function that gets called if the archived type doesn't
// implement CheckBytes.
unsafe fn check_bytes_dyn_unimplemented(
    _bytes: *const u8,
    _context: &mut dyn DynArchiveContext,
) -> Result<(), Box<dyn Error>> {
    Err(Box::new(CheckBytesUnimplemented).into())
}

#[doc(hidden)]
pub trait NotCheckBytesDyn {
    const CHECK_BYTES_DYN: CheckBytesDyn = check_bytes_dyn_unimplemented;
}

impl<T> NotCheckBytesDyn for T {}

#[doc(hidden)]
pub struct IsCheckBytesDyn<T>(PhantomData<T>);

impl<T: for<'a> CheckBytes<(dyn DynArchiveContext + 'a)>> IsCheckBytesDyn<T> {
    pub const CHECK_BYTES_DYN: CheckBytesDyn = Self::check_bytes_dyn;

    unsafe fn check_bytes_dyn(
        bytes: *const u8,
        context: &mut dyn DynArchiveContext,
    ) -> Result<(), Box<dyn Error>> {
        T::check_bytes(bytes, context)?;
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplValidation {
    pub size: usize,
    pub align: usize,
    pub check_bytes_dyn: CheckBytesDyn,
}

#[doc(hidden)]
#[macro_export]
macro_rules! validation {
    ($type:ty) => {{
        use rkyv_dyn::validation::{ImplValidation, IsCheckBytesDyn, NotCheckBytesDyn};
        ImplValidation {
            size: core::mem::size_of::<$type>(),
            align: core::mem::align_of::<$type>(),
            check_bytes_dyn: IsCheckBytesDyn::<$type>::CHECK_BYTES_DYN,
        }
    }};
}

/// Errors that can occur when checking archived trait objects
#[derive(Debug)]
pub enum ArchivedDynError {
    /// The trait object has an invalid impl id or was stomped by vtable caching
    InvalidImplId(u64),
}

impl fmt::Display for ArchivedDynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedDynError::InvalidImplId(id) => {
                if id & 1 == 0 {
                    write!(f, "invalid impl id: overwritten with vtable pointer")
                } else {
                    write!(f, "invalid impl id: {} not registered", id)
                }
            }
        }
    }
}

impl Error for ArchivedDynError {}

impl From<Unreachable> for ArchivedDynError {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<T: TypeName + ?Sized, C: ?Sized> CheckBytes<C> for ArchivedDyn<T> {
    type Error = ArchivedDynError;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        RelPtr::check_bytes(bytes.add(offset_of!(Self, ptr)), context)?;
        let type_id = AtomicU64::check_bytes(bytes.add(offset_of!(Self, type_id)), context)?.load(Ordering::Relaxed);
        PhantomData::<T>::check_bytes(bytes.add(offset_of!(Self, _phantom)), context)?;
        if IMPL_REGISTRY.data::<T>(type_id).is_some() {
            Ok(&*bytes.cast())
        } else {
            Err(ArchivedDynError::InvalidImplId(type_id))
        }
    }
}

/// Errors that can occur when checking archived trait objects
#[derive(Debug)]
pub enum CheckArchivedDynError {
    /// The trait object has an invalid impl id or was stomped by vtable caching
    CheckBytes(Box<dyn Error>),
}

impl fmt::Display for CheckArchivedDynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckArchivedDynError::CheckBytes(e) => write!(f, "check bytes: {}", e),
        }
    }
}

impl Error for CheckArchivedDynError {}

impl From<Box<dyn Error>> for CheckArchivedDynError {
    fn from(e: Box<dyn Error>) -> Self {
        Self::CheckBytes(e)
    }
}

impl<T: TypeName + ?Sized, C: ArchiveBoundsContext + ArchiveMemoryContext> CheckBytesRef<C> for ArchivedDyn<T> {
    type RefError = CheckArchivedDynError;
    type Target = T;

    fn check_ptr(&self, context: &mut C) -> Result<(*const u8, usize), ArchiveBoundsError> {
        CheckBytesRef::<dyn DynArchiveContext>::check_ptr(self, context)
    }

    unsafe fn check_ref_bytes<'a>(&'a self, bytes: *const u8, context: &mut C) -> Result<&'a Self::Target, Self::RefError> {
        CheckBytesRef::<dyn DynArchiveContext>::check_ref_bytes(self, bytes, context)
    }
}

impl<'c, T: TypeName + ?Sized> CheckBytesRef<(dyn DynArchiveContext + 'c)> for ArchivedDyn<T> {
    type RefError = CheckArchivedDynError;
    type Target = T;

    fn check_ptr(&self, context: &mut (dyn DynArchiveContext + 'c)) -> Result<(*const u8, usize), ArchiveBoundsError> {
        let impl_data = IMPL_REGISTRY.data::<T>(self.type_id.load(Ordering::Relaxed)).unwrap();
        let ref_bytes = unsafe { context.check_rel_ptr(&self.ptr, impl_data.validation.size, impl_data.validation.align)? };
        Ok((ref_bytes, impl_data.validation.size))
    }

    unsafe fn check_ref_bytes<'a>(&'a self, bytes: *const u8, context: &mut (dyn DynArchiveContext + 'c)) -> Result<&'a Self::Target, Self::RefError> {
        let impl_data = IMPL_REGISTRY.data::<T>(self.type_id.load(Ordering::Relaxed)).unwrap();
        let check_bytes_dyn = impl_data.validation.check_bytes_dyn;
        check_bytes_dyn(bytes, context)?;
        Ok(&**self)
    }
}
