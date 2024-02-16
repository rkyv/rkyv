//! Validators add validation capabilities by wrapping and extending basic
//! validators.

use core::{any::TypeId, fmt};

use bytecheck::rancor::Error;
use rancor::fail;

use crate::validation::SharedContext;

#[cfg(not(feature = "std"))]
use hashbrown::HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

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
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedError::TypeMismatch { previous, current } => write!(
                f,
                "the same memory region has been claimed as two different types ({:?} and {:?})",
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

/// A validator that can verify shared memory.
#[derive(Debug, Default)]
pub struct SharedValidator {
    shared: HashMap<usize, TypeId>,
}

impl SharedValidator {
    /// Wraps the given context and adds shared memory validation.
    #[inline]
    pub fn new() -> Self {
        Self {
            // TODO: consider deferring this to avoid the overhead of
            // constructing
            shared: HashMap::new(),
        }
    }

    /// Shared memory validator with specific capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            shared: HashMap::with_capacity(capacity),
        }
    }
}

impl<E: Error> SharedContext<E> for SharedValidator {
    #[inline]
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E> {
        #[cfg(not(feature = "std"))]
        use hashbrown::hash_map::Entry;
        #[cfg(feature = "std")]
        use std::collections::hash_map::Entry;

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
