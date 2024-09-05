use core::{error::Error, fmt, hash::BuildHasherDefault};

use hashbrown::hash_map::{Entry, HashMap};
use rancor::{fail, Source};

use crate::{
    hash::FxHasher64,
    ser::{sharing::SharingState, Sharing},
};

/// A shared pointer strategy that shares serializations of the same shared
/// pointer.
#[derive(Debug, Default)]
pub struct Share {
    shared_address_to_pos:
        HashMap<usize, Option<usize>, BuildHasherDefault<FxHasher64>>,
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

#[derive(Debug)]
struct NotStarted;

impl fmt::Display for NotStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shared pointer was not started sharing")
    }
}

impl Error for NotStarted {}

#[derive(Debug)]
struct AlreadyFinished;

impl fmt::Display for AlreadyFinished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shared pointer was already finished sharing")
    }
}

impl Error for AlreadyFinished {}

impl<E: Source> Sharing<E> for Share {
    fn start_sharing(&mut self, address: usize) -> SharingState {
        match self.shared_address_to_pos.entry(address) {
            Entry::Vacant(vacant) => {
                vacant.insert(None);
                SharingState::Started
            }
            Entry::Occupied(occupied) => {
                if let Some(pos) = occupied.get() {
                    SharingState::Finished(*pos)
                } else {
                    SharingState::Pending
                }
            }
        }
    }

    fn finish_sharing(&mut self, address: usize, pos: usize) -> Result<(), E> {
        match self.shared_address_to_pos.entry(address) {
            Entry::Vacant(_) => fail!(NotStarted),
            Entry::Occupied(mut occupied) => {
                let inner = occupied.get_mut();
                if inner.is_some() {
                    fail!(AlreadyFinished);
                } else {
                    *inner = Some(pos);
                    Ok(())
                }
            }
        }
    }
}
