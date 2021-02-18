//! Deserialization traits, deserializers, and adapters.

#[cfg(feature = "std")]
pub mod adapters;
pub mod deserializers;

use core::alloc;
use crate::{ArchiveUnsized, DeserializeUnsized, Fallible};

/// A context that provides a memory allocator.
///
/// Most types that support [`DeserializeRef`] will require this kind of
/// context.
pub trait Deserializer: Fallible {
    /// Allocates and returns memory with the given layout.
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error>;
}

pub trait SharedPointer {
    fn data_address(&self) -> *const ();
}

/// A context that provides shared memory support.
///
/// Shared pointers require this kind of context to deserialize.
pub trait SharedDeserializer: Deserializer {
    /// Checks whether the given reference has been deserialized and either
    /// clones the existing shared pointer to it, or deserializes it and uses
    /// `to_shared` to create a shared pointer.
    fn deserialize_shared<T: ArchiveUnsized + ?Sized, P: 'static + SharedPointer, F: FnOnce(*mut T) -> P>(&mut self, value: &T::Archived, to_shared: F) -> Result<*const T, Self::Error>
    where
        T::Archived: DeserializeUnsized<T, Self>;
}
