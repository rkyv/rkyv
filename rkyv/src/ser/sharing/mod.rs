//! Shared pointer serialization.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;

use rancor::{Fallible, Strategy};

use crate::SerializeUnsized;

/// A shared pointer serialization strategy.
///
/// This trait is required to serialize `Rc` and `Arc`.
pub trait Sharing<E = <Self as Fallible>::Error> {
    /// Gets the position of a serialized shared pointer by address.
    ///
    /// Returns `None` if the pointer has not yet been added.
    fn get_shared_ptr(&self, address: usize) -> Option<usize>;

    /// Adds the serialized position of a shared pointer.
    fn add_shared_ptr(&mut self, address: usize, pos: usize) -> Result<(), E>;
}

impl<T, E> Sharing<E> for Strategy<T, E>
where
    T: Sharing<E> + ?Sized,
{
    fn get_shared_ptr(&self, address: usize) -> Option<usize> {
        T::get_shared_ptr(self, address)
    }

    fn add_shared_ptr(&mut self, address: usize, pos: usize) -> Result<(), E> {
        T::add_shared_ptr(self, address, pos)
    }
}

/// TODO: Document this
pub trait SharingExt<E>: Sharing<E> {
    /// Gets the position of a previously-added shared value.
    ///
    /// Returns `None` if the value has not yet been added.
    #[inline]
    fn get_shared<T: ?Sized>(&self, value: &T) -> Option<usize> {
        self.get_shared_ptr(value as *const T as *const () as usize)
    }

    /// Adds the position of a shared value to the registry.
    #[inline]
    fn add_shared<T: ?Sized>(
        &mut self,
        value: &T,
        pos: usize,
    ) -> Result<(), E> {
        self.add_shared_ptr(value as *const T as *const () as usize, pos)
    }

    /// Archives the given shared value and returns its position. If the value
    /// has already been added then it returns the position of the
    /// previously added value.
    #[inline]
    fn serialize_shared<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, <Self as Fallible>::Error>
    where
        Self: Fallible<Error = E>,
    {
        if let Some(pos) = self.get_shared(value) {
            Ok(pos)
        } else {
            let pos = value.serialize_unsized(self)?;
            self.add_shared(value, pos)?;
            Ok(pos)
        }
    }
}

impl<S, E> SharingExt<E> for S where S: Sharing<E> + ?Sized {}
