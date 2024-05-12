//! Adapters wrap deserializers and add support for deserializer traits.

use core::{fmt, mem::size_of};
#[cfg(feature = "std")]
use std::collections::hash_map;

#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Source};

use super::{ErasedPtr, Pooling};

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

#[derive(Debug)]
struct SharedPointer {
    ptr: ErasedPtr,
    drop: unsafe fn(ErasedPtr),
}

impl Drop for SharedPointer {
    fn drop(&mut self) {
        unsafe {
            (self.drop)(self.ptr);
        }
    }
}

/// A shared pointer strategy that pools together deserializations of the same
/// shared pointer.
#[derive(Default)]
pub struct Pool {
    shared_pointers: hash_map::HashMap<usize, SharedPointer>,
}

impl Pool {
    /// Creates a new shared pointer unifier.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new shared pointer unifier with initial capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shared_pointers: hash_map::HashMap::with_capacity(capacity),
        }
    }
}

impl fmt::Debug for Pool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.shared_pointers.iter()).finish()
    }
}

impl<E: Source> Pooling<E> for Pool {
    fn get_shared_ptr(&mut self, address: usize) -> Option<ErasedPtr> {
        self.shared_pointers.get(&address).map(|p| p.ptr)
    }

    unsafe fn add_shared_ptr(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), E> {
        match self.shared_pointers.entry(address) {
            hash_map::Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer { address });
            }
            hash_map::Entry::Vacant(e) => {
                e.insert(SharedPointer { ptr, drop });
                Ok(())
            }
        }
    }
}
