//! APIs for testing code that uses rkyv.
//!
//! These APIs are test-only. The exact signatures of these APIs change
//! depending on which features are enabled so that they can be used uniformly
//! across:
//!
//! - `std`, no-std, and no-std-no-alloc configurations
//! - `bytecheck` enabled or disabled
//!
//! In the no-std-no-alloc configuration, the amount of data that can be
//! serialized or allocated during serialization is limited. If you test in
//! these configurations, keep your data sizes relatively small.

#[cfg(feature = "alloc")]
mod outer_high;
#[cfg(not(feature = "alloc"))]
mod outer_low;

#[cfg(feature = "bytecheck")]
mod inner_checked;
#[cfg(not(feature = "bytecheck"))]
mod inner_unchecked;

#[cfg(feature = "bytecheck")]
pub use self::inner_checked::*;
#[cfg(not(feature = "bytecheck"))]
pub use self::inner_unchecked::*;
#[cfg(feature = "alloc")]
pub use self::outer_high::*;
#[cfg(not(feature = "alloc"))]
pub use self::outer_low::*;
