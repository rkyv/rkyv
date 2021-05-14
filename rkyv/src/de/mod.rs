//! Deserialization traits, deserializers, and adapters.

#[cfg(feature = "std")]
pub mod adapters;

use crate::{ArchiveUnsized, DeserializeUnsized, Fallible};

/// A deserializable shared pointer type.
pub trait SharedPointer {
    /// Returns the data address for this shared pointer.
    fn data_address(&self) -> *const ();
}

/// A context that provides shared memory support.
///
/// Shared pointers require this kind of context to deserialize.
pub trait SharedDeserializer: Fallible {
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
