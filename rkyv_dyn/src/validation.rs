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

impl<T: ArchiveMemoryContext + ?Sized> DynArchiveContext for T {
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

// This is the fallback function that gets called if the archived type doesn't
// implement CheckBytes.
unsafe fn check_rel_ptr_unimplemented(
    _rel_ptr: &RelPtr,
    _context: &mut dyn DynArchiveContext,
) -> Result<(), Box<dyn Error>> {
    Err(Box::new(CheckBytesUnimplemented).into())
}

type FnCheckRelPtr = unsafe fn(&RelPtr, &mut dyn DynArchiveContext) -> Result<(), Box<dyn Error>>;

#[doc(hidden)]
pub trait NotCheckBytesDyn {
    const CHECK_REL_PTR: FnCheckRelPtr = check_rel_ptr_unimplemented;
}

impl<T> NotCheckBytesDyn for T {}

#[doc(hidden)]
pub struct IsCheckBytesDyn<T>(PhantomData<T>);

impl<T: for<'a> CheckBytes<(dyn DynArchiveContext + 'a)>> IsCheckBytesDyn<T> {
    pub const CHECK_REL_PTR: FnCheckRelPtr = Self::check_rel_ptr_dyn;

    unsafe fn check_rel_ptr_dyn(
        rel_ptr: &RelPtr,
        context: &mut dyn DynArchiveContext,
    ) -> Result<(), Box<dyn Error>> {
        let data = context.claim::<T>(rel_ptr, 1)?;
        T::check_bytes(data, context)?;
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplValidation {
    pub size: usize,
    pub align: usize,
    pub check_rel_ptr: FnCheckRelPtr,
}

#[doc(hidden)]
#[macro_export]
macro_rules! validation {
    ($type:ty) => {{
        use rkyv_dyn::validation::{ImplValidation, IsCheckBytesDyn, NotCheckBytesDyn};
        ImplValidation {
            size: core::mem::size_of::<$type>(),
            align: core::mem::align_of::<$type>(),
            check_rel_ptr: IsCheckBytesDyn::<$type>::CHECK_REL_PTR,
        }
    }};
}

/// Errors that can occur when checking archived trait objects
#[derive(Debug)]
pub enum ArchivedDynError {
    /// The trait object has an invalid impl id or was stomped by vtable caching
    InvalidImplId(u64),
    /// A memory error occurred while checking the data for the trait object
    MemoryError(ArchiveMemoryError),
    /// The trait object check failed or the type does not implement CheckBytes
    CheckBytes(Box<dyn Error>),
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
            ArchivedDynError::MemoryError(e) => write!(f, "archived dyn memory error: {}", e),
            ArchivedDynError::CheckBytes(e) => write!(f, "invalid trait object: {}", e),
        }
    }
}

impl Error for ArchivedDynError {}

impl From<ArchiveMemoryError> for ArchivedDynError {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl From<Unreachable> for ArchivedDynError {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<T: TypeName + ?Sized, C: ArchiveMemoryContext> CheckBytes<C> for ArchivedDyn<T> {
    type Error = ArchivedDynError;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        <Self as CheckBytes<dyn DynArchiveContext>>::check_bytes(bytes, context)
    }
}

impl<'b, T: TypeName + ?Sized> CheckBytes<dyn DynArchiveContext + 'b> for ArchivedDyn<T> {
    type Error = ArchivedDynError;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut (dyn DynArchiveContext + 'b),
    ) -> Result<&'a Self, Self::Error> {
        let archived_type_id =
            AtomicU64::check_bytes(bytes.add(offset_of!(Self, type_id)), context)?;
        let type_id = archived_type_id.load(Ordering::Relaxed);
        if let Some(impl_data) = IMPL_REGISTRY.data::<T>(type_id) {
            let rel_ptr = RelPtr::check_bytes(bytes.add(offset_of!(Self, ptr)), context)?;
            let check_rel_ptr = impl_data.validation.check_rel_ptr;
            check_rel_ptr(rel_ptr, context).map_err(ArchivedDynError::CheckBytes)?;
            #[cfg(feature = "vtable_cache")]
            archived_type_id.store(impl_data.vtable.0 as usize as u64, Ordering::Relaxed);
            Ok(&*bytes.cast())
        } else {
            Err(ArchivedDynError::InvalidImplId(type_id))
        }
    }
}
