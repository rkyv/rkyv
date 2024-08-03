//! Validators add validation capabilities by wrapping and extending basic
//! validators.

use core::{any::TypeId, fmt, hash::BuildHasherDefault};
#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use hashbrown::HashMap;
use rancor::{fail, Source};

use crate::{hash::FxHasher64, validation::SharedContext};

/// Errors that can occur when checking shared memory.
#[derive(Debug)]
pub enum SharedError {
    /// Multiple pointers exist to the same location with different types
    TypeMismatch {
        /// A previous type that the location was checked as
        previous: TypeId,
        /// The current type that the location is checked as
        current: TypeId,
    },
}

impl fmt::Display for SharedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedError::TypeMismatch { previous, current } => write!(
                f,
                "the same memory region has been claimed as two different \
                 types ({:?} and {:?})",
                previous, current
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SharedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SharedError::TypeMismatch { .. } => None,
        }
    }
}

/// A validator that can verify shared pointers.
#[derive(Debug, Default)]
pub struct SharedValidator {
    shared: HashMap<usize, TypeId, BuildHasherDefault<FxHasher64>>,
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
            shared: HashMap::with_capacity_and_hasher(
                capacity,
                Default::default(),
            ),
        }
    }
}

impl<E: Source> SharedContext<E> for SharedValidator {
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E> {
        #[cfg(feature = "std")]
        use std::collections::hash_map::Entry;

        #[cfg(not(feature = "std"))]
        use hashbrown::hash_map::Entry;

        match self.shared.entry(address) {
            Entry::Occupied(previous_type_entry) => {
                let previous_type_id = previous_type_entry.get();
                if previous_type_id != &type_id {
                    fail!(SharedError::TypeMismatch {
                        previous: *previous_type_id,
                        current: type_id,
                    })
                } else {
                    Ok(false)
                }
            }
            Entry::Vacant(ent) => {
                ent.insert(type_id);
                Ok(true)
            }
        }
    }
}
