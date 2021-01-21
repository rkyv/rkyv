#[cfg(feature = "std")]
pub mod adapters;
pub mod deserializers;

use core::alloc;
use crate::{ArchiveRef, DeserializeRef, Fallible};

pub trait Deserializer: Fallible {
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error>;
}

pub trait SharedDeserializer: Deserializer {
    fn deserialize_shared<T: ArchiveRef + ?Sized, P: Clone + 'static>(&mut self, reference: &T::Reference, to_shared: impl FnOnce(*mut T) -> P) -> Result<P, Self::Error>
    where
        T::Reference: DeserializeRef<T, Self>;
}
