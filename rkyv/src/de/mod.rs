//! Deserialization traits, deserializers, and adapters.

#[cfg(feature = "std")]
pub mod adapters;
pub mod deserializers;

use core::alloc;
use crate::{ArchiveRef, DeserializeRef, Fallible};

/// A context that provides a memory allocator.
///
/// Most types that support [`DeserializeRef`] will require this kind of
/// context.
pub trait Deserializer: Fallible {
    /// Allocates and returns memory with the given layout.
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error>;
}

/// A context that provides shared memory support.
///
/// Shared pointers require this kind of context to deserialize.
pub trait SharedDeserializer: Deserializer {
    /// Checks whether the given reference has been deserialized and either
    /// clones the existing shared pointer to it, or deserializes it and uses
    /// `to_shared` to create a shared pointer.
    fn deserialize_shared<T: ArchiveRef + ?Sized, P: Clone + 'static>(&mut self, reference: &T::Reference, to_shared: impl FnOnce(*mut T) -> P) -> Result<P, Self::Error>
    where
        T::Reference: DeserializeRef<T, Self>;
}
