//! Deserializers that can be used standalone and provide basic capabilities.

use ::core::{
    any::{type_name, TypeId},
    error::Error,
    fmt,
    mem::transmute,
};
use ptr_meta::{from_raw_parts_mut, metadata, DynMetadata, Pointee};
use rancor::{fail, Source};

/// Type-erased pointer metadata.
#[derive(Clone, Copy)]
pub union Metadata {
    unit: (),
    usize: usize,
    vtable: DynMetadata<()>,
}

impl fmt::Debug for Metadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<metadata>")
    }
}

impl From<()> for Metadata {
    fn from(value: ()) -> Self {
        Self { unit: value }
    }
}

impl From<usize> for Metadata {
    fn from(value: usize) -> Self {
        Self { usize: value }
    }
}

impl<T: ?Sized> From<DynMetadata<T>> for Metadata {
    fn from(value: DynMetadata<T>) -> Self {
        Self {
            vtable: unsafe {
                transmute::<DynMetadata<T>, DynMetadata<()>>(value)
            },
        }
    }
}

/// A type which can be extracted from `Metadata`.
pub trait FromMetadata {
    /// Extracts this type from [`Metadata`].
    ///
    /// # Safety
    ///
    /// The metadata must have been created by calling `Metadata::from` on a
    /// value of this type.
    unsafe fn from_metadata(metadata: Metadata) -> Self;
}

impl FromMetadata for () {
    unsafe fn from_metadata(metadata: Metadata) -> Self {
        unsafe { metadata.unit }
    }
}

impl FromMetadata for usize {
    unsafe fn from_metadata(metadata: Metadata) -> Self {
        unsafe { metadata.usize }
    }
}

impl<T: ?Sized> FromMetadata for DynMetadata<T> {
    unsafe fn from_metadata(metadata: Metadata) -> Self {
        unsafe { transmute::<DynMetadata<()>, DynMetadata<T>>(metadata.vtable) }
    }
}

/// A type-erased pointer.
#[derive(Clone, Copy, Debug)]
pub struct ErasedPtr {
    data_address: *mut (),
    metadata: Metadata,
    shared_pointer_type: Option<SharedPointerType>,
}

#[derive(Clone, Copy, Debug)]
struct SharedPointerType {
    pointee_type_id: TypeId,
    pointer_type_id: TypeId,
    pointee_type_name: &'static str,
    pointer_type_name: &'static str,
}

impl SharedPointerType {
    fn of<T: ?Sized + 'static, P: 'static>() -> Self {
        Self {
            pointee_type_id: TypeId::of::<T>(),
            pointer_type_id: TypeId::of::<P>(),
            pointee_type_name: type_name::<T>(),
            pointer_type_name: type_name::<P>(),
        }
    }

    fn matches<T: ?Sized + 'static, P: 'static>(&self) -> bool {
        self.pointee_type_id == TypeId::of::<T>()
            && self.pointer_type_id == TypeId::of::<P>()
    }
}

#[derive(Debug)]
struct SharedPointerTypeMismatch {
    previous_pointee: &'static str,
    previous_pointer: &'static str,
    current_pointee: &'static str,
    current_pointer: &'static str,
}

impl fmt::Display for SharedPointerTypeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "shared pointer was already deserialized as {} pointing to {}, \
             but was later requested as {} pointing to {}",
            self.previous_pointer,
            self.previous_pointee,
            self.current_pointer,
            self.current_pointee,
        )
    }
}

impl Error for SharedPointerTypeMismatch {}

impl ErasedPtr {
    /// Returns an erased pointer corresponding to the given pointer.
    #[inline]
    pub fn new<T>(ptr: *mut T) -> Self
    where
        T: Pointee + ?Sized,
        T::Metadata: Into<Metadata>,
    {
        Self {
            data_address: ptr.cast(),
            metadata: metadata(ptr).into(),
            shared_pointer_type: None,
        }
    }

    pub(crate) fn new_shared<T, P>(ptr: *mut T) -> Self
    where
        T: Pointee + ?Sized + 'static,
        T::Metadata: Into<Metadata>,
        P: 'static,
    {
        Self {
            data_address: ptr.cast(),
            metadata: metadata(ptr).into(),
            shared_pointer_type: Some(SharedPointerType::of::<T, P>()),
        }
    }

    /// Returns the data address corresponding to this erased pointer.
    #[inline]
    pub fn data_address(&self) -> *mut () {
        self.data_address
    }

    /// Returns the metadata associated with this erased pointer.
    #[inline]
    pub fn metadata(&self) -> Metadata {
        self.metadata
    }

    pub(crate) fn check_shared_pointer_type<T, P, E>(&self) -> Result<(), E>
    where
        T: ?Sized + 'static,
        P: 'static,
        E: Source,
    {
        match self.shared_pointer_type {
            Some(shared_pointer_type)
                if shared_pointer_type.matches::<T, P>() =>
            {
                Ok(())
            }
            Some(shared_pointer_type) => fail!(SharedPointerTypeMismatch {
                previous_pointee: shared_pointer_type.pointee_type_name,
                previous_pointer: shared_pointer_type.pointer_type_name,
                current_pointee: type_name::<T>(),
                current_pointer: type_name::<P>(),
            }),
            None => fail!(SharedPointerTypeMismatch {
                previous_pointee: "<unknown>",
                previous_pointer: "<unknown>",
                current_pointee: type_name::<T>(),
                current_pointer: type_name::<P>(),
            }),
        }
    }

    /// # Safety
    ///
    /// `self` must be created from a valid pointer to `T`.
    #[inline]
    pub unsafe fn downcast_unchecked<T>(&self) -> *mut T
    where
        T: Pointee + ?Sized,
        T::Metadata: FromMetadata,
    {
        from_raw_parts_mut(self.data_address, unsafe {
            T::Metadata::from_metadata(self.metadata)
        })
    }
}
