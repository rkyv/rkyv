//! Deserializers that can be used standalone and provide basic capabilities.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

use ::core::{
    alloc::LayoutError, error::Error, fmt, mem::transmute, ptr::NonNull,
};
use ptr_meta::{from_raw_parts_mut, metadata, DynMetadata, Pointee};
use rancor::{fail, Fallible, ResultExt as _, Source, Strategy};

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;
use crate::{traits::LayoutRaw, ArchiveUnsized, DeserializeUnsized};

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

// These impls are sound because `Metadata` has the type-level invariant that
// `From` will only be called on `Metadata` created from pointers with the
// corresponding metadata.

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
    data_address: NonNull<()>,
    metadata: Metadata,
}

impl ErasedPtr {
    /// Returns an erased pointer corresponding to the given pointer.
    #[inline]
    pub fn new<T>(ptr: NonNull<T>) -> Self
    where
        T: Pointee + ?Sized,
        T::Metadata: Into<Metadata>,
    {
        Self {
            data_address: ptr.cast(),
            metadata: metadata(ptr.as_ptr()).into(),
        }
    }

    /// Returns the data address corresponding to this erased pointer.
    #[inline]
    pub fn data_address(&self) -> *mut () {
        self.data_address.as_ptr()
    }

    /// # Safety
    ///
    /// `self` must be created from a valid pointer to `T`.
    #[inline]
    unsafe fn downcast_unchecked<T>(&self) -> *mut T
    where
        T: Pointee + ?Sized,
        T::Metadata: FromMetadata,
    {
        from_raw_parts_mut(self.data_address.as_ptr(), unsafe {
            T::Metadata::from_metadata(self.metadata)
        })
    }
}

/// A deserializable shared pointer type.
///
/// # Safety
///
/// `alloc` and `from_value` must return pointers which are non-null, writeable,
/// and properly aligned for `T`.
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

/// The result of starting to deserialize a shared pointer.
pub enum PoolingState {
    /// The caller started pooling this value. They should proceed to
    /// deserialize the shared value and call `finish_pooling`.
    Started,
    /// Another caller started pooling this value, but has not finished yet.
    /// This can only occur with cyclic shared pointer structures, and so rkyv
    /// treats this as an error by default.
    Pending,
    /// This value has already been pooled. The caller should use the returned
    /// pointer to pool its value.
    Finished(ErasedPtr),
}

/// A shared pointer deserialization strategy.
///
/// This trait is required to deserialize `Rc` and `Arc`.
pub trait Pooling<E = <Self as Fallible>::Error> {
    /// Starts pooling the value associated with the given address.
    fn start_pooling(&mut self, address: usize) -> PoolingState;

    /// Finishes pooling the value associated with the given address.
    ///
    /// Returns an error if the given address was not pending.
    ///
    /// # Safety
    ///
    /// The given `drop` function must be valid to call with `ptr`.
    unsafe fn finish_pooling(
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
    fn start_pooling(&mut self, address: usize) -> PoolingState {
        T::start_pooling(self, address)
    }

    unsafe fn finish_pooling(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), E> {
        // SAFETY: The safety requirements for `finish_pooling` are the same as
        // the requirements for calling this function.
        unsafe { T::finish_pooling(self, address, ptr, drop) }
    }
}

#[derive(Debug)]
struct CyclicSharedPointerError;

impl fmt::Display for CyclicSharedPointerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "encountered cyclic shared pointers while deserializing\nhelp: \
             change your deserialization strategy to `Unpool` or use the \
             `Unpool` wrapper type to break the cycle",
        )
    }
}

impl Error for CyclicSharedPointerError {}

/// Helper methods for [`Pooling`].
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
        T::Metadata: Into<Metadata> + FromMetadata,
        T::Archived: DeserializeUnsized<T, Self>,
        P: SharedPointer<T>,
        Self: Fallible<Error = E>,
        E: Source,
    {
        unsafe fn drop_shared<T, P>(ptr: ErasedPtr)
        where
            T: Pointee + ?Sized,
            T::Metadata: FromMetadata,
            P: SharedPointer<T>,
        {
            unsafe { P::drop(ptr.downcast_unchecked::<T>()) }
        }

        let address = value as *const T::Archived as *const () as usize;
        let metadata = T::Archived::deserialize_metadata(value);

        match self.start_pooling(address) {
            PoolingState::Started => {
                let out = P::alloc(metadata).into_error()?;
                unsafe { value.deserialize_unsized(self, out)? };
                let ptr = unsafe { NonNull::new_unchecked(P::from_value(out)) };

                unsafe {
                    self.finish_pooling(
                        address,
                        ErasedPtr::new(ptr),
                        drop_shared::<T, P>,
                    )?;
                }

                Ok(ptr.as_ptr())
            }
            PoolingState::Pending => fail!(CyclicSharedPointerError),
            PoolingState::Finished(ptr) => {
                Ok(from_raw_parts_mut(ptr.data_address.as_ptr(), metadata))
            }
        }
    }
}

impl<T, E> PoolingExt<E> for T where T: Pooling<E> + ?Sized {}
