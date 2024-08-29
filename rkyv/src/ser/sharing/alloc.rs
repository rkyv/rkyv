use core::{fmt, hash::BuildHasherDefault, mem::size_of};

use hashbrown::hash_map::{Entry, HashMap};
use rancor::{fail, Source};

use crate::{hash::FxHasher64, ser::Sharing};

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
    shared_address_to_pos:
        HashMap<usize, usize, BuildHasherDefault<FxHasher64>>,
}

impl Share {
    /// Creates a new shared pointer unifier.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new shared pointer unifier with initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shared_address_to_pos: HashMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
        }
    }

    /// Clears the shared pointer unifier for reuse.
    pub fn clear(&mut self) {
        self.shared_address_to_pos.clear();
    }
}

impl<E: Source> Sharing<E> for Share {
    fn get_shared_ptr(&self, address: usize) -> Option<usize> {
        self.shared_address_to_pos.get(&address).copied()
    }

    fn add_shared_ptr(&mut self, address: usize, pos: usize) -> Result<(), E> {
        match self.shared_address_to_pos.entry(address) {
            Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer { address });
            }
            Entry::Vacant(e) => {
                e.insert(pos);
                Ok(())
            }
        }
    }
}
