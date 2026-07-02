//! Shared pointer validation.

#[cfg(feature = "alloc")]
mod validator;

use core::any::TypeId;

use rancor::{Fallible, Strategy};

#[cfg(feature = "alloc")]
pub use self::validator::*;
use crate::de::{ErasedPtr, Metadata};

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
    /// Starts validating the value associated with an erased data pointer and
    /// shared pointer type.
    ///
    /// The arguments to this method are relatively complex:
    ///
    /// - `shared_type_id` is the type of the shared pointer to start validating
    /// - `ptr` is an erased pointer to the data in the buffer that is shared
    /// - `metadata_is_eq` is a comparison function called on potentially-equal
    ///   data pointers
    ///
    /// `shared_type_id` and `ptr` are used as a unique key to identify the
    /// shared pointer. If the shared context finds another shared pointer with
    /// the same shared type ID and data address, it will call `metadata_is_eq`
    /// on the metadata of the existing shared pointer and the metadata of the
    /// provided shared pointer.
    ///
    /// Returns an error if the value associated with the given address was
    /// started with a different type ID.
    fn start_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
        metadata_is_eq: unsafe fn(Metadata, Metadata) -> bool,
    ) -> Result<ValidationState, E>;

    /// Finishes validating the value associated with the given address.
    ///
    /// Returns an error if the given address was not pending.
    fn finish_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
    ) -> Result<(), E>;
}

impl<T, E> SharedContext<E> for Strategy<T, E>
where
    T: SharedContext<E>,
{
    fn start_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
        metadata_is_eq: unsafe fn(Metadata, Metadata) -> bool,
    ) -> Result<ValidationState, E> {
        T::start_shared(self, shared_type_id, ptr, metadata_is_eq)
    }

    fn finish_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
    ) -> Result<(), E> {
        T::finish_shared(self, shared_type_id, ptr)
    }
}
