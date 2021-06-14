//! Trait implementations for `core`, `alloc`, and `std` types.

#[cfg(feature = "alloc")]
pub mod alloc;
pub mod core;
#[cfg(feature = "std")]
pub mod std;
