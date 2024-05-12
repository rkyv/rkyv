use core::{fmt, mem::size_of};
#[cfg(feature = "std")]
use std::collections::hash_map;

#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Source};

use crate::ser::Sharing;

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

/// A shared pointer strategy that shares serializations of the same shared
/// pointer.
#[derive(Debug, Default)]
pub struct Share {
    shared_address_to_pos: hash_map::HashMap<usize, usize>,
}

impl Share {
    /// Creates a new shared pointer unifier.
    #[inline]
    pub fn new() -> Self {
        Self {
            shared_address_to_pos: hash_map::HashMap::new(),
        }
    }

    /// Creates a new shared pointer unifier with initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shared_address_to_pos: hash_map::HashMap::with_capacity(capacity),
        }
    }
}

impl<E: Source> Sharing<E> for Share {
    fn get_shared_ptr(&self, address: usize) -> Option<usize> {
        self.shared_address_to_pos.get(&address).copied()
    }

    fn add_shared_ptr(&mut self, address: usize, pos: usize) -> Result<(), E> {
        match self.shared_address_to_pos.entry(address) {
            hash_map::Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer { address });
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(pos);
                Ok(())
            }
        }
    }
}
