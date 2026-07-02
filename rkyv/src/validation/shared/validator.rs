//! Validators add validation capabilities by wrapping and extending basic
//! validators.

use core::{any::TypeId, error::Error, fmt, hash::BuildHasherDefault};
#[cfg(feature = "std")]
use std::collections::hash_map;

#[cfg(not(feature = "std"))]
use hashbrown::hash_map;
use rancor::{fail, Source};

use crate::{
    erased::{ErasedPtr, Metadata},
    hash::FxHasher64,
    validation::{shared::ValidationState, SharedContext},
};

#[derive(Debug)]
struct SharedValidationState {
    type_id: TypeId,
    metadata: Metadata,
    is_finished: bool,
}

/// A validator that can verify shared pointers.
#[derive(Debug, Default)]
pub struct SharedValidator {
    shared: hash_map::HashMap<
        usize,
        SharedValidationState,
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
struct MetadataMismatch;

impl fmt::Display for MetadataMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "the same memory region has been claimed as the same type with
            different pointer metadata (e.g. slice length)",
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

impl<E: Source> SharedContext<E> for SharedValidator {
    fn start_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
        metadata_is_eq: unsafe fn(Metadata, Metadata) -> bool,
    ) -> Result<ValidationState, E> {
        match self.shared.entry(ptr.data_address() as usize) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(SharedValidationState {
                    type_id: shared_type_id,
                    metadata: ptr.metadata(),
                    is_finished: false,
                });
                Ok(ValidationState::Started)
            }
            hash_map::Entry::Occupied(occupied) => {
                let state = occupied.get();
                if state.type_id != shared_type_id {
                    fail!(TypeMismatch {
                        previous: state.type_id,
                        current: shared_type_id,
                    });
                }

                let is_same_metadata =
                    unsafe { metadata_is_eq(ptr.metadata(), state.metadata) };
                if !is_same_metadata {
                    fail!(MetadataMismatch);
                }

                if !state.is_finished {
                    Ok(ValidationState::Pending)
                } else {
                    Ok(ValidationState::Finished)
                }
            }
        }
    }

    fn finish_shared(
        &mut self,
        _shared_type_id: TypeId,
        ptr: ErasedPtr,
    ) -> Result<(), E> {
        match self.shared.entry(ptr.data_address() as usize) {
            hash_map::Entry::Vacant(_) => fail!(NotStarted),
            hash_map::Entry::Occupied(mut occupied) => {
                let state = occupied.get_mut();

                if state.is_finished {
                    fail!(AlreadyFinished);
                }

                state.is_finished = true;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(any(
        feature = "pointer_width_16",
        feature = "pointer_width_64"
    )))]
    #[test]
    fn conflicting_metadata() {
        use rancor::Error;

        use super::*;
        use crate::{
            alloc::rc::Rc, api::high::access, util::Align, Archive, Serialize,
        };

        #[expect(dead_code)]
        #[derive(Archive, Serialize)]
        #[rkyv(crate, derive(Debug))]
        struct Test {
            a: Rc<[u8]>,
            b: Rc<[u8]>,
        }

        // Invalid archive (mismatched metadata)
        let synthetic_buf = Align([
            // Shared slice
            1u8, 2u8, 3u8, 4u8, // First Rc
            0xfc, 0xff, 0xff, 0xff, // points 4 bytes backward
            4u8, 0u8, 0u8, 0u8, // slice is 4 bytes long
            // Second Rc
            0xf4, 0xff, 0xff, 0xff, // points 12 bytes backward
            2u8, 0u8, 0u8, 0u8, // slice is 2 bytes long
        ]);

        let result = access::<ArchivedTest, Error>(&*synthetic_buf);
        assert_source!(result.unwrap_err(), MetadataMismatch);
    }
}
