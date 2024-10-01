//! Shared pointer validation.

#[cfg(feature = "alloc")]
mod validator;

use core::any::TypeId;

use rancor::{Fallible, Strategy};

#[cfg(feature = "alloc")]
pub use self::validator::*;

/// The result of starting to validate a shared pointer.
pub enum ValidationState {
    /// The caller started validating this value. They should proceed to check
    /// the shared value and call `finish_shared`.
    Started,
    /// Another caller started validating this value, but has not finished yet.
    /// This can only occur with cyclic shared pointer structures, and so rkyv
    /// treats this as an error by default.
    Pending,
    /// This value has already been validated.
    Finished,
}

/// A context that can validate shared archive memory.
///
/// Shared pointers require this kind of context to validate.
pub trait SharedContext<E = <Self as Fallible>::Error> {
    /// Starts validating the value associated with the given address.
    ///
    /// Returns an error if the value associated with the given address was
    /// started with a different type ID.
    fn start_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<ValidationState, E>;

    /// Finishes validating the value associated with the given address.
    ///
    /// Returns an error if the given address was not pending.
    fn finish_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<(), E>;
}

impl<T, E> SharedContext<E> for Strategy<T, E>
where
    T: SharedContext<E>,
{
    fn start_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<ValidationState, E> {
        T::start_shared(self, address, type_id)
    }

    fn finish_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<(), E> {
        T::finish_shared(self, address, type_id)
    }
}
