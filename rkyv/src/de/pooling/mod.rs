//! Deserializers that can be used standalone and provide basic capabilities.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

use ::core::{alloc::LayoutError, fmt, mem::transmute};
use ptr_meta::{from_raw_parts_mut, metadata, DynMetadata, Pointee};
use rancor::{Fallible, ResultExt as _, Source, Strategy};

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;
use crate::{ArchiveUnsized, DeserializeUnsized, LayoutRaw};

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

// These impls are sound because `Metadata` has the type-level invariant that
// `From` will only be called on `Metadata` created from pointers with the
// corresponding metadata.

impl From<Metadata> for () {
    fn from(value: Metadata) -> Self {
        unsafe { value.unit }
    }
}

impl From<Metadata> for usize {
    fn from(value: Metadata) -> Self {
        unsafe { value.usize }
    }
}

impl<T: ?Sized> From<Metadata> for DynMetadata<T> {
    fn from(value: Metadata) -> Self {
        unsafe { transmute::<DynMetadata<()>, DynMetadata<T>>(value.vtable) }
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

    /// # Safety
    ///
    /// `self` must be created from a valid pointer to `T`.
    #[inline]
    unsafe fn downcast_unchecked<T>(&self) -> *mut T
    where
        T: Pointee + ?Sized,
        Metadata: Into<T::Metadata>,
    {
        from_raw_parts_mut(self.data_address, self.metadata.into())
    }
}

/// A deserializable shared pointer type.
///
/// # Safety
///
/// TODO
pub unsafe trait SharedPointer<T: Pointee + ?Sized> {
    /// Allocates space for a value with the given metadata.
    fn alloc(metadata: T::Metadata) -> Result<*mut T, LayoutError>;

    /// Creates a new `Self` from a pointer to a valid `T`.
    ///
    /// # Safety
    ///
    /// `ptr` must have been allocated via `alloc`. `from_value` must not have
    /// been called on `ptr` yet.
    unsafe fn from_value(ptr: *mut T) -> *mut T;

    /// Drops a pointer created by `from_value`.
    ///
    /// # Safety
    ///
    /// - `ptr` must have been created using `from_value`.
    /// - `drop` must only be called once per `ptr`.
    unsafe fn drop(ptr: *mut T);
}

/// A shared pointer deserialization strategy.
///
/// This trait is required to deserialize `Rc` and `Arc`.
pub trait Pooling<E = <Self as Fallible>::Error> {
    /// Gets the data pointer of a previously-deserialized shared pointer.
    fn get_shared_ptr(&mut self, address: usize) -> Option<ErasedPtr>;

    /// Adds the data address of a deserialized shared pointer to the registry.
    ///
    /// # Safety
    ///
    /// The given `drop` function must be valid to call with the given
    /// `pointer`.
    unsafe fn add_shared_ptr(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), E>;
}

impl<T, E> Pooling<E> for Strategy<T, E>
where
    T: Pooling<E>,
{
    fn get_shared_ptr(&mut self, address: usize) -> Option<ErasedPtr> {
        T::get_shared_ptr(self, address)
    }

    unsafe fn add_shared_ptr(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), E> {
        // SAFETY: The safety requirements for `add_shared_ptr` are the same as
        // the requirements for calling this function.
        unsafe { T::add_shared_ptr(self, address, ptr, drop) }
    }
}

/// Helper methods for `SharedDeserializeRegistry`.
pub trait PoolingExt<E>: Pooling<E> {
    /// Checks whether the given reference has been deserialized and either uses
    /// the existing shared pointer to it, or deserializes it and converts
    /// it to a shared pointer with `to_shared`.
    fn deserialize_shared<T, P>(
        &mut self,
        value: &T::Archived,
    ) -> Result<*mut T, Self::Error>
    where
        T: ArchiveUnsized + Pointee + LayoutRaw + ?Sized,
        T::Metadata: Into<Metadata>,
        Metadata: Into<T::Metadata>,
        T::Archived: DeserializeUnsized<T, Self>,
        P: SharedPointer<T>,
        Self: Fallible<Error = E>,
        E: Source,
    {
        unsafe fn drop_shared<T, P>(ptr: ErasedPtr)
        where
            T: Pointee + ?Sized,
            Metadata: Into<T::Metadata>,
            P: SharedPointer<T>,
        {
            unsafe { P::drop(ptr.downcast_unchecked::<T>()) }
        }

        let address = value as *const T::Archived as *const () as usize;
        let metadata = T::Archived::deserialize_metadata(value, self)?;

        if let Some(shared_pointer) = self.get_shared_ptr(address) {
            Ok(from_raw_parts_mut(shared_pointer.data_address, metadata))
        } else {
            let out = P::alloc(metadata).into_error()?;
            unsafe { value.deserialize_unsized(self, out)? };
            let ptr = unsafe { P::from_value(out) };

            unsafe {
                self.add_shared_ptr(
                    address,
                    ErasedPtr::new(ptr),
                    drop_shared::<T, P>,
                )?;
            }

            Ok(ptr)
        }
    }
}

impl<T, E> PoolingExt<E> for T where T: Pooling<E> + ?Sized {}
