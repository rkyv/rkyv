#[cfg(feature = "std")]
use core::alloc;
#[cfg(feature = "std")]
use std::{error::Error, fmt};
#[cfg(feature = "std")]
use crate::{de::Deserializer, Fallible};

#[cfg(feature = "std")]
#[derive(Debug)]
pub enum AllocDeserializerError {}

#[cfg(feature = "std")]
impl fmt::Display for AllocDeserializerError {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!();
    }
}

#[cfg(feature = "std")]
impl Error for AllocDeserializerError {}

#[cfg(feature = "std")]
pub struct AllocDeserializer;

#[cfg(feature = "std")]
impl Fallible for AllocDeserializer {
    type Error = AllocDeserializerError;
}

#[cfg(feature = "std")]
impl Deserializer for AllocDeserializer {
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error> {
        Ok(std::alloc::alloc(layout))
    }
}
