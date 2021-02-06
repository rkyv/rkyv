//! Validation implementations and helper types.

use core::{
    alloc::Layout,
    any::TypeId,
    fmt,
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};
use std::{
    collections::HashMap,
    error::Error,
};
use bytecheck::{CheckBytes, Unreachable};
use rkyv::{
    offset_of,
    validation::{
        ArchiveBoundsContext,
        ArchiveMemoryContext,
        SharedArchiveContext,
    },
    Fallible,
};
use rkyv_typename::TypeName;
use crate::{ArchivedDynMetadata, IMPL_REGISTRY, RegisteredImpl, VTable};

pub trait DynContext {
    unsafe fn check_rel_ptr_dyn(
        &mut self,
        base: *const u8,
        offset: isize,
    ) -> Result<*const u8, Box<dyn Error>>;

    unsafe fn bounds_check_ptr_dyn(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), Box<dyn Error>>;

    unsafe fn claim_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
    ) -> Result<(), Box<dyn Error>>;

    unsafe fn claim_shared_bytes_dyn(
        &mut self,
        start: *const u8,
        len: usize,
        type_id: TypeId
    ) -> Result<bool, Box<dyn Error>>;
}

impl<C: ArchiveBoundsContext + ArchiveMemoryContext + SharedArchiveContext + ?Sized> DynContext for C
where
    C::Error: Error,
{
    unsafe fn check_rel_ptr_dyn(&mut self, base: *const u8, offset: isize) -> Result<*const u8, Box<dyn Error>> {
        self.check_rel_ptr(base, offset).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    unsafe fn bounds_check_ptr_dyn(&mut self, ptr: *const u8, layout: &Layout) -> Result<(), Box<dyn Error>> {
        self.bounds_check_ptr(ptr, layout).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    unsafe fn claim_bytes_dyn(&mut self, start: *const u8, len: usize) -> Result<(), Box<dyn Error>> {
        self.claim_bytes(start, len).map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    unsafe fn claim_shared_bytes_dyn(&mut self, start: *const u8, len: usize, type_id: TypeId) -> Result<bool, Box<dyn Error>> {
        self.claim_shared_bytes(start, len, type_id).map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}

impl Fallible for (dyn DynContext + '_) {
    type Error = Box<dyn Error>;
}

impl ArchiveBoundsContext for (dyn DynContext + '_) {
    unsafe fn check_rel_ptr(&mut self, base: *const u8, offset: isize) -> Result<*const u8, Self::Error> {
        self.check_rel_ptr_dyn(base, offset)
    }

    unsafe fn bounds_check_ptr(&mut self, ptr: *const u8, layout: &Layout) -> Result<(), Self::Error> {
        self.bounds_check_ptr_dyn(ptr, layout)
    }
}

impl ArchiveMemoryContext for (dyn DynContext + '_) {
    unsafe fn claim_bytes(&mut self, start: *const u8, len: usize) -> Result<(), Self::Error> {
        self.claim_bytes_dyn(start, len)
    }
}

impl SharedArchiveContext for (dyn DynContext + '_) {
    unsafe fn claim_shared_bytes(&mut self, start: *const u8, len: usize, type_id: TypeId) -> Result<bool, Box<dyn Error>> {
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
    }
}

/// Errors that can occur when checking archived trait objects
#[derive(Debug)]
pub enum DynMetadataError {
    /// The trait object has an invalid impl id or was stomped by vtable caching
    InvalidImplId(u64),
}

impl fmt::Display for DynMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DynMetadataError::InvalidImplId(id) => {
                if id & 1 == 0 {
                    write!(f, "invalid impl id: overwritten with vtable pointer")
                } else {
                    write!(f, "invalid impl id: {} not registered", id)
                }
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

        let type_id = AtomicU64::check_bytes(bytes.add(offset_of!(Self, type_id)).cast(), context)?.load(Ordering::Relaxed);
        PhantomData::<T>::check_bytes(bytes.add(offset_of!(Self, phantom)).cast(), context)?;
        if type_id & 1 == 0 || IMPL_REGISTRY.get::<T>(type_id).is_some() {
            Ok(&*value)
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
            CheckDynError::CheckBytes(e) => Some(&**e),
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
    vtable: VTable,
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

pub struct CheckBytesRegistry {
    vtable_to_check_bytes: HashMap<VTable, ImplValidation>,
}

impl CheckBytesRegistry {
    fn new() -> Self {
        Self {
            vtable_to_check_bytes: HashMap::new(),
        }
    }

    fn add_entry(&mut self, entry: &CheckBytesEntry) {
        let old_value = self.vtable_to_check_bytes.insert(entry.vtable, entry.validation);

        debug_assert!(old_value.is_none(), "vtable conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    pub fn get(&self, vtable: VTable) -> Option<&ImplValidation> {
        self.vtable_to_check_bytes.get(&vtable)
    }
}

lazy_static::lazy_static! {
    pub static ref CHECK_BYTES_REGISTRY: CheckBytesRegistry =  {
        let mut result = CheckBytesRegistry::new();
        for entry in inventory::iter::<CheckBytesEntry> {
            result.add_entry(entry);
        }
        result
    };
}

#[macro_export]
macro_rules! register_validation {
    ($type:ty as $trait:ty) => {
        use rkyv_dyn::validation::{CheckBytesEntry, IsCheckBytesDyn, NotCheckBytesDyn};

        inventory::submit! { CheckBytesEntry::new::<$type, $trait>(IsCheckBytesDyn::<$type>::CHECK_BYTES_DYN) }
    }
}
