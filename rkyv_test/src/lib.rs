#![cfg_attr(not(feature = "std"), no_std)]
#![allow(dead_code)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "alloc")]
mod test_alloc;
#[cfg(feature = "std")]
mod test_std;
pub mod util;
#[cfg(feature = "bytecheck")]
pub mod validation;
