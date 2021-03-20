//! Deserializers that can be used standalone and provide basic capabilities.

#[cfg(feature = "std")]
use crate::{de::Deserializer, Fallible, Unreachable};
#[cfg(feature = "std")]
use core::alloc;

/// A deserializer that provides access to the global alloc function.
#[cfg(feature = "std")]
pub struct AllocDeserializer;

#[cfg(feature = "std")]
impl Fallible for AllocDeserializer {
    type Error = Unreachable;
}

#[cfg(feature = "std")]
impl Deserializer for AllocDeserializer {
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error> {
        Ok(std::alloc::alloc(layout))
    }
}
