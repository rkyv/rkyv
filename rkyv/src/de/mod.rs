//! Deserialization traits, deserializers, and adapters.

pub mod pooling;

use rancor::Strategy;

#[doc(inline)]
pub use self::pooling::*;

/// A deserializer suitable for environments where allocations cannot be made.
pub type CoreDeserializer<E> = Strategy<Unpool, E>;

/// A general-purpose deserializer suitable for environments where allocations
/// can be made.
#[cfg(feature = "alloc")]
pub type DefaultDeserializer<E> = Strategy<Pool, E>;
