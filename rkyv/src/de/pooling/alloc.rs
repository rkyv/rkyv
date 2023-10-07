//! Adapters wrap deserializers and add support for deserializer traits.

use super::{Pooling, SharedPointer};
#[cfg(not(feature = "std"))]
use alloc::boxed::Box;
use core::{fmt, mem::size_of};
#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Error};
#[cfg(feature = "std")]
use std::collections::hash_map;

#[derive(Debug)]
struct DuplicateSharedPointer {
    address: usize,
}

impl fmt::Display for DuplicateSharedPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "duplicate shared pointer: {:#.*x}",
            size_of::<usize>() * 2,
            self.address
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DuplicateSharedPointer {}

/// A shared pointer strategy that unifies deserializations of the same shared
/// pointer.
pub struct Unify {
    shared_pointers: hash_map::HashMap<usize, Box<dyn SharedPointer>>,
}

impl Unify {
    /// Creates a new shared pointer unifier.
    #[inline]
    pub fn new() -> Self {
        Self {
            shared_pointers: hash_map::HashMap::new(),
        }
    }

    /// Creates a new shared pointer unifier with initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shared_pointers: hash_map::HashMap::with_capacity(capacity),
        }
    }
}

impl fmt::Debug for Unify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map()
            .entries(
                self.shared_pointers
                    .iter()
                    .map(|(s, p)| (s, &**p as *const _)),
            )
            .finish()
    }
}

impl Default for Unify {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Error> Pooling<E> for Unify {
    fn get_shared_ptr(&mut self, address: usize) -> Option<&dyn SharedPointer> {
        self.shared_pointers.get(&address).map(|p| p.as_ref())
    }

    fn add_shared_ptr(
        &mut self,
        address: usize,
        shared: Box<dyn SharedPointer>,
    ) -> Result<(), E> {
        match self.shared_pointers.entry(address) {
            hash_map::Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer { address });
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(shared);
                Ok(())
            }
        }
    }
}
