use crate::ser::Sharing;
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

/// A shared pointer strategy that unifies serializations of the same shared
/// pointer.
#[derive(Debug)]
pub struct Unify {
    shared_ptr_to_pos: hash_map::HashMap<*const u8, usize>,
}

// TODO: get rid of these by replacing pointer with usize in key of shared_ptr_to_pos

// SAFETY: SharedSerializeMap is safe to send to another thread
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Send for Unify {}

// SAFETY: SharedSerializeMap is safe to share between threads
// This trait is not automatically implemented because the struct contains a pointer
unsafe impl Sync for Unify {}

impl Unify {
    /// Creates a new shared registry map.
    #[inline]
    pub fn new() -> Self {
        Self {
            shared_ptr_to_pos: hash_map::HashMap::new(),
        }
    }
}

impl Default for Unify {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Error> Sharing<E> for Unify {
    fn get_shared_ptr(&self, value: *const u8) -> Option<usize> {
        self.shared_ptr_to_pos.get(&value).copied()
    }

    fn add_shared_ptr(
        &mut self,
        value: *const u8,
        pos: usize,
    ) -> Result<(), E> {
        match self.shared_ptr_to_pos.entry(value) {
            hash_map::Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer {
                    address: value as usize
                });
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(pos);
                Ok(())
            }
        }
    }
}
