//! Deserializers that can be used standalone and provide basic capabilities.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;
