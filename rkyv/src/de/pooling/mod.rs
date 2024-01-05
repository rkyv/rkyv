//! Deserializers that can be used standalone and provide basic capabilities.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;

#[cfg(feature = "alloc")]
use crate::{ArchiveUnsized, DeserializeUnsized};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use ::alloc::boxed::Box;
#[cfg(feature = "alloc")]
use ::core::alloc::Layout;
use rancor::{Fallible, Strategy};

/// A deserializable shared pointer type.
#[cfg(feature = "alloc")]
pub trait SharedPointer {
    /// Returns the data address for this shared pointer.
    fn data_address(&self) -> *const ();
}

/// A shared pointer deserialization strategy.
///
/// This trait is required to deserialize `Rc` and `Arc`.
#[cfg(feature = "alloc")]
pub trait Pooling<E = <Self as Fallible>::Error> {
    /// Gets the data pointer of a previously-deserialized shared pointer.
    fn get_shared_ptr(&mut self, address: usize) -> Option<&dyn SharedPointer>;

    /// Adds the data address of a deserialized shared pointer to the registry.
    fn add_shared_ptr(
        &mut self,
        address: usize,
        shared: Box<dyn SharedPointer>,
    ) -> Result<(), E>;
}

impl<T, E> Pooling<E> for Strategy<T, E>
where
    T: Pooling<E>,
{
    #[inline]
    fn get_shared_ptr(&mut self, address: usize) -> Option<&dyn SharedPointer> {
        T::get_shared_ptr(self, address)
    }

    #[inline]
    fn add_shared_ptr(
        &mut self,
        address: usize,
        shared: Box<dyn SharedPointer>,
    ) -> Result<(), E> {
        T::add_shared_ptr(self, address, shared)
    }
}

/// Helper methods for `SharedDeserializeRegistry`.
pub trait PoolingExt<E>: Pooling<E> {
    /// Checks whether the given reference has been deserialized and either uses the existing shared
    /// pointer to it, or deserializes it and converts it to a shared pointer with `to_shared`.
    #[inline]
    fn deserialize_shared<T, P, F, A>(
        &mut self,
        value: &T::Archived,
        to_shared: F,
        alloc: A,
    ) -> Result<*const T, Self::Error>
    where
        T: ArchiveUnsized + ?Sized,
        T::Archived: DeserializeUnsized<T, Self>,
        P: SharedPointer + 'static,
        F: FnOnce(*mut T) -> P,
        A: FnMut(Layout) -> *mut u8,
        Self: Fallible<Error = E>,
    {
        let address = value as *const T::Archived as *const () as usize;
        let metadata = T::Archived::deserialize_metadata(value, self)?;

        if let Some(shared_pointer) = self.get_shared_ptr(address) {
            Ok(ptr_meta::from_raw_parts(
                shared_pointer.data_address(),
                metadata,
            ))
        } else {
            let deserialized_data =
                unsafe { value.deserialize_unsized(self, alloc)? };
            let shared_ptr = to_shared(ptr_meta::from_raw_parts_mut(
                deserialized_data,
                metadata,
            ));
            let data_address = shared_ptr.data_address();

            self.add_shared_ptr(
                address,
                Box::new(shared_ptr) as Box<dyn SharedPointer>,
            )?;
            Ok(ptr_meta::from_raw_parts(data_address, metadata))
        }
    }
}

impl<T, E> PoolingExt<E> for T where T: Pooling<E> + ?Sized {}
