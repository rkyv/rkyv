//! Shared pointer serialization.

#[cfg(feature = "alloc")]
mod alloc;
mod core;

use ::core::{error::Error, fmt};
use rancor::{fail, Fallible, Source, Strategy};

#[cfg(feature = "alloc")]
pub use self::alloc::*;
pub use self::core::*;
use crate::SerializeUnsized;

/// The result of starting to serialize a shared pointer.
pub enum SharingState {
    /// The caller started sharing this value. They should proceed to serialize
    /// the shared value and call `finish_sharing`.
    Started,
    /// Another caller started sharing this value, but has not finished yet.
    /// This can only occur with cyclic shared pointer structures, and so rkyv
    /// treats this as an error by default.
    Pending,
    /// This value has already been shared. The caller should use the returned
    /// address to share its value.
    Finished(usize),
}

/// A shared pointer serialization strategy.
///
/// This trait is required to serialize `Rc` and `Arc`.
pub trait Sharing<E = <Self as Fallible>::Error> {
    /// Starts sharing the value associated with the given address.
    fn start_sharing(&mut self, address: usize) -> SharingState;

    /// Finishes sharing the value associated with the given address.
    ///
    /// Returns an error if the given address was not pending.
    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), E>;
}

impl<T, E> Sharing<E> for &mut T
where
    T: Sharing<E> + ?Sized,
{
    fn start_sharing(&mut self, address: usize) -> SharingState {
        T::start_sharing(*self, address)
    }

    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), E> {
        T::finish_sharing(*self, address, pos)
    }
}

impl<T, E> Sharing<E> for Strategy<T, E>
where
    T: Sharing<E> + ?Sized,
{
    fn start_sharing(&mut self, address: usize) -> SharingState {
        T::start_sharing(self, address)
    }

    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), E> {
        T::finish_sharing(self, address, pos)
    }
}

#[derive(Debug)]
struct CyclicSharedPointerError;

impl fmt::Display for CyclicSharedPointerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "encountered cyclic shared pointers while serializing\nhelp: \
             change your serialization strategy to `Unshare` or use the \
             `Unshare` wrapper type to break the cycle",
        )
    }
}

impl Error for CyclicSharedPointerError {}

/// Helper methods for [`Sharing`].
pub trait SharingExt<E>: Sharing<E> {
    /// Serializes the given shared value and returns its position. If the value
    /// has already been serialized then it returns the position of the
    /// previously added value.
    ///
    /// Returns an error if cyclic shared pointers are encountered.
    fn serialize_shared<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, <Self as Fallible>::Error>
    where
        Self: Fallible<Error = E>,
        E: Source,
    {
        let addr = value as *const T as *const () as usize;
        match self.start_sharing(addr) {
            SharingState::Started => {
                let pos = value.serialize_unsized(self)?;
                self.finish_sharing(addr, pos)?;
                Ok(pos)
            }
            SharingState::Pending => fail!(CyclicSharedPointerError),
            SharingState::Finished(pos) => Ok(pos),
        }
    }
}

impl<S, E> SharingExt<E> for S where S: Sharing<E> + ?Sized {}
