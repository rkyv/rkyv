use core::{error::Error, fmt, hash::BuildHasherDefault};

use hashbrown::hash_map::{Entry, HashMap};
use rancor::{fail, Source};

use crate::{
    de::pooling::{ErasedPtr, Pooling, PoolingState},
    hash::FxHasher64,
};

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
        HashMap<usize, Option<SharedPointer>, BuildHasherDefault<FxHasher64>>,
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

#[derive(Debug)]
struct NotStarted;

impl fmt::Display for NotStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shared pointer was not started pooling")
    }
}

impl Error for NotStarted {}

#[derive(Debug)]
struct AlreadyFinished;

impl fmt::Display for AlreadyFinished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shared pointer was already finished pooling")
    }
}

impl Error for AlreadyFinished {}

impl<E: Source> Pooling<E> for Pool {
    fn start_pooling(&mut self, address: usize) -> PoolingState {
        match self.shared_pointers.entry(address) {
            Entry::Vacant(vacant) => {
                vacant.insert(None);
                PoolingState::Started
            }
            Entry::Occupied(occupied) => {
                if let Some(shared) = occupied.get() {
                    PoolingState::Finished(shared.ptr)
                } else {
                    PoolingState::Pending
                }
            }
        }
    }

    unsafe fn finish_pooling(
        &mut self,
        address: usize,
        ptr: ErasedPtr,
        drop: unsafe fn(ErasedPtr),
    ) -> Result<(), E> {
        match self.shared_pointers.entry(address) {
            Entry::Vacant(_) => fail!(NotStarted),
            Entry::Occupied(mut occupied) => {
                let inner = occupied.get_mut();
                if inner.is_some() {
                    fail!(AlreadyFinished)
                } else {
                    *inner = Some(SharedPointer { ptr, drop });
                    Ok(())
                }
            }
        }
    }
}
