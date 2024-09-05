//! Validators add validation capabilities by wrapping and extending basic
//! validators.

use core::{any::TypeId, error::Error, fmt, hash::BuildHasherDefault};
#[cfg(feature = "std")]
use std::collections::hash_map;

#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Source};

use crate::{
    hash::FxHasher64,
    validation::{shared::ValidationState, SharedContext},
};

/// A validator that can verify shared pointers.
#[derive(Debug, Default)]
pub struct SharedValidator {
    shared: hash_map::HashMap<
        usize,
        (TypeId, bool),
        BuildHasherDefault<FxHasher64>,
    >,
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

impl<E: Source> SharedContext<E> for SharedValidator {
    fn start_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<ValidationState, E> {
        match self.shared.entry(address) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert((type_id, false));
                Ok(ValidationState::Started)
            }
            hash_map::Entry::Occupied(occupied) => {
                let (previous_type_id, finished) = occupied.get();
                if previous_type_id != &type_id {
                    fail!(TypeMismatch {
                        previous: *previous_type_id,
                        current: type_id,
                    })
                } else if !finished {
                    Ok(ValidationState::Pending)
                } else {
                    Ok(ValidationState::Finished)
                }
            }
        }
    }

    fn finish_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<(), E> {
        match self.shared.entry(address) {
            hash_map::Entry::Vacant(_) => fail!(NotStarted),
            hash_map::Entry::Occupied(mut occupied) => {
                let (previous_type_id, finished) = occupied.get_mut();
                if previous_type_id != &type_id {
                    fail!(TypeMismatch {
                        previous: *previous_type_id,
                        current: type_id,
                    });
                } else if *finished {
                    fail!(AlreadyFinished);
                } else {
                    *finished = true;
                    Ok(())
                }
            }
        }
    }
}
