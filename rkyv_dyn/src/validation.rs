//! Validation implementations and helper types.

use crate::{ArchivedDynMetadata, RegisteredImpl, IMPL_REGISTRY};
use bytecheck::{CheckBytes, Unreachable};
use core::{
    alloc::Layout,
    any::TypeId,
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};
use rkyv::{
    offset_of,
    validation::{ArchiveBoundsContext, ArchiveMemoryContext, SharedArchiveContext},
    Fallible,
};
use rkyv_typename::TypeName;
use std::{collections::HashMap, error::Error};

/// A context that's object safe and suitable for checking most types.
pub trait DynContext {
    /// Checks the given parts of a relative pointer for bounds issues.
    unsafe fn check_rel_ptr_dyn(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Box<dyn Error>>;

    /// Checks the given memory block for bounds issues.
    unsafe fn bounds_check_ptr_dyn(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Box<dyn Error>>;

    /// Claims `count` bytes located `offset` bytes away from `base`.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
    ) -> Result<(), Box<dyn Error>>;

    /// Claims `count` shared bytes located `offset` bytes away from `base`.
    ///
    /// Returns whether the bytes need to be checked.
    ///
    /// # Safety
    ///
    /// `base` must be inside the archive this context was created for.
    unsafe fn claim_shared_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId,
    ) -> Result<bool, Box<dyn Error>>;
}

impl<C: ArchiveBoundsContext + ArchiveMemoryContext + SharedArchiveContext + ?Sized> DynContext
    for C
where
    C::Error: Error,
{
    unsafe fn check_rel_ptr_dyn(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Box<dyn Error>> {
        self.check_rel_ptr(base, offset)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    unsafe fn bounds_check_ptr_dyn(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Box<dyn Error>> {
        self.bounds_check_ptr(ptr, layout)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    unsafe fn claim_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
    ) -> Result<(), Box<dyn Error>> {
        self.claim_bytes(start, len)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    unsafe fn claim_shared_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId,
    ) -> Result<bool, Box<dyn Error>> {
        self.claim_shared_bytes(start, len, type_id)
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

impl Fallible for (dyn DynContext + '_) {
    type Error = Box<dyn Error>;
}

impl ArchiveBoundsContext for (dyn DynContext + '_) {
    unsafe fn check_rel_ptr(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Self::Error> {
        self.check_rel_ptr_dyn(base, offset)
    }

    unsafe fn bounds_check_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Self::Error> {
        self.bounds_check_ptr_dyn(ptr, layout)
    }
}

impl ArchiveMemoryContext for (dyn DynContext + '_) {
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error> {
        self.claim_bytes_dyn(start, len)
    }
}

impl SharedArchiveContext for (dyn DynContext + '_) {
    unsafe fn claim_shared_bytes(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId,
    ) -> Result<bool, Box<dyn Error>> {
        self.claim_shared_bytes_dyn(start, len, type_id)
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

type CheckBytesDyn = unsafe fn(*const u8, &mut dyn DynContext) -> Result<(), Box<dyn Error>>;

// This is the fallback function that gets called if the archived type doesn't
// implement CheckBytes.
unsafe fn check_bytes_dyn_unimplemented(
    _bytes: *const u8,
    _context: &mut dyn DynContext,
) -> Result<(), Box<dyn Error>> {
    Err(Box::new(CheckBytesUnimplemented).into())
}

#[doc(hidden)]
pub trait NotCheckBytesDyn {
    const CHECK_BYTES_DYN: CheckBytesDyn = check_bytes_dyn_unimplemented;
}

impl<T: ?Sized> NotCheckBytesDyn for T {}

#[doc(hidden)]
pub struct IsCheckBytesDyn<T: ?Sized>(PhantomData<T>);

impl<T: for<'a> CheckBytes<dyn DynContext + 'a>> IsCheckBytesDyn<T> {
    pub const CHECK_BYTES_DYN: CheckBytesDyn = Self::check_bytes_dyn;

    unsafe fn check_bytes_dyn<'a>(
        bytes: *const u8,
        context: &mut dyn DynContext,
    ) -> Result<(), Box<dyn Error>> {
        T::check_bytes(bytes.cast(), context)?;
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplValidation {
    pub layout: Layout,
    pub check_bytes_dyn: CheckBytesDyn,
}

#[doc(hidden)]
#[macro_export]
macro_rules! validation {
    ($type:ty as $trait:ty) => {
        use rkyv_dyn::validation::{ImplValidation, IsCheckBytesDyn, NotCheckBytesDyn};
    };
}

/// Errors that can occur when checking archived trait objects
#[derive(Debug)]
pub enum DynMetadataError {
    /// The trait object has an invalid type id
    InvalidImplId(u64),
    /// The cached vtable does not match the vtable for the type id
    MismatchedCachedVtable {
        /// The type id of the trait object
        type_id: u64,
        /// The expected vtable
        expected: usize,
        /// The found vtable
        found: usize,
    },
}

impl fmt::Display for DynMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DynMetadataError::InvalidImplId(id) => {
                write!(f, "invalid impl id: {} not registered", id)
            }
            DynMetadataError::MismatchedCachedVtable {
                type_id,
                expected,
                found,
            } => {
                write!(
                    f,
                    "mismatched cached vtable for {}: expected {} but found {}",
                    type_id, expected, found
                )
            }
        }
    }
}

impl Error for DynMetadataError {}

impl From<Unreachable> for DynMetadataError {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<T: TypeName + ?Sized, C: ?Sized> CheckBytes<C> for ArchivedDynMetadata<T> {
    type Error = DynMetadataError;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();

        let type_id = *u64::check_bytes(bytes.add(offset_of!(Self, type_id)).cast(), context)?;
        PhantomData::<T>::check_bytes(bytes.add(offset_of!(Self, phantom)).cast(), context)?;
        if let Some(impl_data) = IMPL_REGISTRY.get::<T>(type_id) {
            let cached_vtable =
                AtomicU64::check_bytes(bytes.add(offset_of!(Self, cached_vtable)).cast(), context)?
                    .load(Ordering::Relaxed);
            if cached_vtable == 0 || cached_vtable as usize == impl_data.vtable {
                Ok(&*value)
            } else {
                Err(DynMetadataError::MismatchedCachedVtable {
                    type_id,
                    expected: impl_data.vtable,
                    found: cached_vtable as usize,
                })
            }
        } else {
            Err(DynMetadataError::InvalidImplId(type_id))
        }
    }
}

/// Errors that can occur when checking archived trait objects
#[derive(Debug)]
pub enum CheckDynError {
    /// The pointer metadata did not match any registered impl
    InvalidMetadata(u64),
    /// An error occurred while checking the bytes of the trait object
    CheckBytes(Box<dyn Error>),
}

impl fmt::Display for CheckDynError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckDynError::InvalidMetadata(n) => write!(f, "invalid metadata: {}", n),
            CheckDynError::CheckBytes(e) => write!(f, "check bytes: {}", e),
        }
    }
}

impl Error for CheckDynError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CheckDynError::InvalidMetadata(_) => None,
            CheckDynError::CheckBytes(e) => Some(e.as_ref()),
        }
    }
}

impl From<Box<dyn Error>> for CheckDynError {
    fn from(e: Box<dyn Error>) -> Self {
        Self::CheckBytes(e)
    }
}

#[doc(hidden)]
pub struct CheckBytesEntry {
    vtable: usize,
    validation: ImplValidation,
}

impl CheckBytesEntry {
    pub fn new<TY: RegisteredImpl<TR>, TR: ?Sized>(check_bytes_dyn: CheckBytesDyn) -> Self {
        Self {
            vtable: <TY as RegisteredImpl<TR>>::vtable(),
            validation: ImplValidation {
                layout: Layout::new::<TY>(),
                check_bytes_dyn,
            },
        }
    }
}

inventory::collect!(CheckBytesEntry);

#[doc(hidden)]
pub struct CheckBytesRegistry {
    vtable_to_check_bytes: HashMap<usize, ImplValidation>,
}

impl CheckBytesRegistry {
    fn new() -> Self {
        Self {
            vtable_to_check_bytes: HashMap::new(),
        }
    }

    fn add_entry(&mut self, entry: &CheckBytesEntry) {
        let old_value = self
            .vtable_to_check_bytes
            .insert(entry.vtable, entry.validation);

        debug_assert!(old_value.is_none(), "vtable conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    pub fn get(&self, vtable: usize) -> Option<&ImplValidation> {
        self.vtable_to_check_bytes.get(&vtable)
    }
}

lazy_static::lazy_static! {
    #[doc(hidden)]
    pub static ref CHECK_BYTES_REGISTRY: CheckBytesRegistry =  {
        let mut result = CheckBytesRegistry::new();
        for entry in inventory::iter::<CheckBytesEntry> {
            result.add_entry(entry);
        }
        result
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! register_validation {
    ($type:ty as $trait:ty) => {
        use rkyv_dyn::validation::{CheckBytesEntry, IsCheckBytesDyn, NotCheckBytesDyn};

        inventory::submit! { CheckBytesEntry::new::<$type, $trait>(IsCheckBytesDyn::<$type>::CHECK_BYTES_DYN) }
    }
}
