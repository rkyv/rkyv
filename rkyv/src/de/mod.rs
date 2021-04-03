//! Deserialization traits, deserializers, and adapters.

#[cfg(feature = "std")]
pub mod adapters;
pub mod deserializers;

use crate::{ArchiveUnsized, DeserializeUnsized, Fallible};
use core::alloc;

/// A context that provides a memory allocator.
///
/// Most types that support [`DeserializeUnsized`] will require this kind of context.
pub trait Deserializer: Fallible {
    /// Allocates and returns memory with the given layout.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the memory returned by this function is deallocated by the
    /// global allocator.
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error>;
}

/// A deserializable shared pointer type.
pub trait SharedPointer {
    /// Returns the data address for this shared pointer.
    fn data_address(&self) -> *const ();
}

/// A context that provides shared memory support.
///
/// Shared pointers require this kind of context to deserialize.
pub trait SharedDeserializer: Deserializer {
    /// Checks whether the given reference has been deserialized and either uses the existing shared
    /// pointer to it, or deserializes it and converts it to a shared pointer with `to_shared`.
    fn deserialize_shared<
        T: ArchiveUnsized + ?Sized,
        P: SharedPointer + 'static,
        F: FnOnce(*mut T) -> P,
    >(
        &mut self,
        value: &T::Archived,
        to_shared: F,
    ) -> Result<*const T, Self::Error>
    where
        T::Archived: DeserializeUnsized<T, Self>;
}
