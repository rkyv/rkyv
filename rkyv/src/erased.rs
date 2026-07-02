//! Deserializers that can be used standalone and provide basic capabilities.

use ::core::{fmt, mem::transmute};
use ptr_meta::{from_raw_parts_mut, metadata, DynMetadata, Pointee};

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
}

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
