//! Shared pointer validation.

#[cfg(feature = "alloc")]
mod validator;

use core::any::TypeId;

use rancor::{Fallible, Strategy};

#[cfg(feature = "alloc")]
pub use self::validator::*;

/// A context that can validate shared archive memory.
///
/// Shared pointers require this kind of context to validate.
pub trait SharedContext<E = <Self as Fallible>::Error> {
    /// Registers the given `ptr` as a shared pointer with the given type.
    ///
    /// Returns `true` if the pointer was newly-registered and `check_bytes`
    /// should be called.
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E>;
}

impl<T, E> SharedContext<E> for Strategy<T, E>
where
    T: SharedContext<E>,
{
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E> {
        T::register_shared_ptr(self, address, type_id)
    }
}
