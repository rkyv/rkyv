//! Validators add validation capabilities by wrapping and extending basic
//! validators.

use core::{any::TypeId, error::Error, fmt, hash::BuildHasherDefault, mem};
#[cfg(feature = "std")]
use std::collections::hash_map;

#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Source};

use crate::{
    alloc::vec::Vec,
    hash::FxHasher64,
    traits::NoUndef,
    validation::{shared::ValidationState, SharedContext},
};

#[derive(Debug)]
struct SharedPointer {
    type_id: TypeId,
    metadata: Vec<u8>,
    finished: bool,
}

/// A validator that can verify shared pointers.
#[derive(Debug, Default)]
pub struct SharedValidator {
    shared:
        hash_map::HashMap<usize, SharedPointer, BuildHasherDefault<FxHasher64>>,
}

impl SharedValidator {
    /// Creates a new shared pointer validator.
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new shared pointer validator with specific capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shared: hash_map::HashMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
        }
    }
}

#[derive(Debug)]
struct TypeMismatch {
    previous: TypeId,
    current: TypeId,
}

impl fmt::Display for TypeMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the same memory region has been claimed as two different types: \
             {:?} and {:?}",
            self.previous, self.current,
        )
    }
}

impl Error for TypeMismatch {}

#[derive(Debug)]
struct MetadataMismatch;

impl fmt::Display for MetadataMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the same memory region has been claimed with different pointer \
             metadata",
        )
    }
}

impl Error for MetadataMismatch {}

#[derive(Debug)]
struct NotStarted;

impl fmt::Display for NotStarted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shared pointer was not started validation")
    }
}

impl Error for NotStarted {}

#[derive(Debug)]
struct AlreadyFinished;

impl fmt::Display for AlreadyFinished {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "shared pointer was already finished validation")
    }
}

impl Error for AlreadyFinished {}

fn metadata_bytes<M: NoUndef>(metadata: &M) -> &[u8] {
    // SAFETY: `NoUndef` guarantees that all bytes of the metadata are
    // initialized. The returned slice is confined to the lifetime of
    // `metadata`.
    unsafe {
        core::slice::from_raw_parts(
            (metadata as *const M).cast(),
            mem::size_of::<M>(),
        )
    }
}

impl<E: Source> SharedContext<E> for SharedValidator {
    fn start_shared<M>(
        &mut self,
        address: usize,
        type_id: TypeId,
        metadata: &M,
    ) -> Result<ValidationState, E>
    where
        M: NoUndef,
    {
        let metadata = metadata_bytes(metadata);
        match self.shared.entry(address) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(SharedPointer {
                    type_id,
                    metadata: metadata.to_vec(),
                    finished: false,
                });
                Ok(ValidationState::Started)
            }
            hash_map::Entry::Occupied(occupied) => {
                let shared = occupied.get();
                if shared.type_id != type_id {
                    fail!(TypeMismatch {
                        previous: shared.type_id,
                        current: type_id,
                    })
                } else if shared.metadata != metadata {
                    fail!(MetadataMismatch)
                } else if !shared.finished {
                    Ok(ValidationState::Pending)
                } else {
                    Ok(ValidationState::Finished)
                }
            }
        }
    }

    fn finish_shared<M>(
        &mut self,
        address: usize,
        type_id: TypeId,
        metadata: &M,
    ) -> Result<(), E>
    where
        M: NoUndef,
    {
        let metadata = metadata_bytes(metadata);
        match self.shared.entry(address) {
            hash_map::Entry::Vacant(_) => fail!(NotStarted),
            hash_map::Entry::Occupied(mut occupied) => {
                let shared = occupied.get_mut();
                if shared.type_id != type_id {
                    fail!(TypeMismatch {
                        previous: shared.type_id,
                        current: type_id,
                    });
                } else if shared.metadata != metadata {
                    fail!(MetadataMismatch)
                } else if shared.finished {
                    fail!(AlreadyFinished);
                } else {
                    shared.finished = true;
                    Ok(())
                }
            }
        }
    }
}
