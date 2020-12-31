//! Validation implementations and helper types.

use super::{ArchivedDyn, IMPL_REGISTRY};
use bytecheck::{CheckBytes, Unreachable};
use core::{
    fmt,
    marker::PhantomData,
    mem,
    sync::atomic::{AtomicU64, Ordering},
};
use rkyv::{
    offset_of,
    validation::{ArchiveContext, ArchiveMemoryError},
    RelPtr,
};
use rkyv_typename::TypeName;
use std::error::Error;

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
    _context: &mut ArchiveContext,
) -> Result<(), Box<dyn Error>> {
    Err(Box::new(CheckBytesUnimplemented).into())
}

type FnCheckRelPtr = unsafe fn(&RelPtr, &mut ArchiveContext) -> Result<(), Box<dyn Error>>;

#[doc(hidden)]
pub trait NotCheckBytesDyn {
    const CHECK_REL_PTR: FnCheckRelPtr = check_rel_ptr_unimplemented;
}

impl<T> NotCheckBytesDyn for T {}

#[doc(hidden)]
pub struct IsCheckBytesDyn<T>(PhantomData<T>);

impl<T: CheckBytes<ArchiveContext>> IsCheckBytesDyn<T> {
    pub const CHECK_REL_PTR: FnCheckRelPtr = Self::check_bytes_dyn;

    unsafe fn check_bytes_dyn(
        rel_ptr: &RelPtr,
        context: &mut ArchiveContext,
    ) -> Result<(), Box<dyn Error>> {
        let data = context.claim_bytes(
            (rel_ptr as *const RelPtr).cast(),
            rel_ptr.offset(),
            mem::size_of::<T>(),
            mem::align_of::<T>(),
        )?;
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

impl<T: TypeName + ?Sized> CheckBytes<ArchiveContext> for ArchivedDyn<T> {
    type Error = ArchivedDynError;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        let type_id = AtomicU64::check_bytes(bytes.add(offset_of!(Self, type_id)), context)?;
        let type_id = type_id.load(Ordering::Relaxed);
        if let Some(vtable_data) = IMPL_REGISTRY.data::<T>(type_id) {
            let rel_ptr = RelPtr::check_bytes(bytes.add(offset_of!(Self, ptr)), context)?;
            let check_rel_ptr = vtable_data.validation.check_rel_ptr;
            check_rel_ptr(rel_ptr, context).map_err(ArchivedDynError::CheckBytes)?;
            #[cfg(feature = "vtable_cache")]
            vtable.store(vtable_data.vtable.0 as usize as u64, Ordering::Relaxed);
            Ok(&*bytes.cast())
        } else {
            Err(ArchivedDynError::InvalidImplId(type_id))
        }
    }
}
