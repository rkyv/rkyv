use core::{fmt, hash::BuildHasherDefault, mem::size_of};

use hashbrown::hash_map::{Entry, HashMap};
use rancor::{fail, Source};

use crate::{
    de::pooling::{ErasedPtr, Pooling},
    hash::FxHasher64,
};

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
    shared_pointers:
        HashMap<usize, SharedPointer, BuildHasherDefault<FxHasher64>>,
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
            shared_pointers: HashMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
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
            Entry::Occupied(_) => {
                fail!(DuplicateSharedPointer { address });
            }
            Entry::Vacant(e) => {
                e.insert(SharedPointer { ptr, drop });
                Ok(())
            }
        }
    }
}
