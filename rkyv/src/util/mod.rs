//! Utilities for common operations.

#[cfg(feature = "alloc")]
mod alloc;
mod inline_vec;
mod ser_vec;

use core::ops::{Deref, DerefMut};

#[doc(inline)]
#[cfg(feature = "alloc")]
pub use self::alloc::*;
#[doc(inline)]
pub use self::{inline_vec::InlineVec, ser_vec::SerVec};

/// A wrapper which aligns its inner value to 16 bytes.
#[derive(Clone, Copy, Debug)]
#[repr(C, align(16))]
pub struct Align<T: ?Sized>(
    /// The inner value.
    pub T,
);

impl<T: ?Sized> Deref for Align<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: ?Sized> DerefMut for Align<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
