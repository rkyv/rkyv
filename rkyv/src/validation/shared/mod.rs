//! Shared pointer validation.

#[cfg(feature = "alloc")]
mod validator;

use core::any::TypeId;

use rancor::{Fallible, Strategy};

#[cfg(feature = "alloc")]
pub use self::validator::*;
use crate::traits::NoUndef;

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
    /// started with a different type ID or pointer metadata.
    fn start_shared<M>(
        &mut self,
        address: usize,
        type_id: TypeId,
        metadata: &M,
    ) -> Result<ValidationState, E>
    where
        M: NoUndef;

    /// Finishes validating the value associated with the given address.
    ///
    /// Returns an error if the given address was not pending, or if the type
    /// ID or pointer metadata does not match the corresponding `start_shared`
    /// call.
    fn finish_shared<M>(
        &mut self,
        address: usize,
        type_id: TypeId,
        metadata: &M,
    ) -> Result<(), E>
    where
        M: NoUndef;
}

impl<T, E> SharedContext<E> for Strategy<T, E>
where
    T: SharedContext<E>,
{
    fn start_shared<M>(
        &mut self,
        address: usize,
        type_id: TypeId,
        metadata: &M,
    ) -> Result<ValidationState, E>
    where
        M: NoUndef,
    {
        T::start_shared(self, address, type_id, metadata)
    }

    fn finish_shared<M>(
        &mut self,
        address: usize,
        type_id: TypeId,
        metadata: &M,
    ) -> Result<(), E>
    where
        M: NoUndef,
    {
        T::finish_shared(self, address, type_id, metadata)
    }
}
