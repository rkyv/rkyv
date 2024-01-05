//! Adapters wrap deserializers and add support for deserializer traits.

use crate::de::{Pooling, SharedPointer};
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
    shared_pointers: hash_map::HashMap<*const u8, Box<dyn SharedPointer>>,
}

impl Unify {
    /// Wraps the given deserializer and adds shared memory support.
    #[inline]
    pub fn new() -> Self {
        Self {
            shared_pointers: hash_map::HashMap::new(),
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
    fn get_shared_ptr(&mut self, ptr: *const u8) -> Option<&dyn SharedPointer> {
        self.shared_pointers.get(&ptr).map(|p| p.as_ref())
    }

    fn add_shared_ptr(
        &mut self,
        ptr: *const u8,
        shared: Box<dyn SharedPointer>,
    ) -> Result<(), E> {
        match self.shared_pointers.entry(ptr) {
            hash_map::Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer {
                    address: ptr as usize
                });
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(shared);
                Ok(())
            }
        }
    }
}
