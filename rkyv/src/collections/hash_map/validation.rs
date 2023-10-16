//! Validation implementation for ArchiveHashMap.

use crate::{
    collections::{
        hash_map::ArchivedHashMap,
        util::Entry,
        ArchivedHashIndex,
    },
    validation::ArchiveContext,
    RelPtr,
};
use bytecheck::CheckBytes;
use core::{
    alloc::{Layout, LayoutError},
    fmt,
    hash::Hash,
    ptr,
};

/// Errors that can occur while checking an archived hash map.
#[derive(Debug)]
pub enum HashMapError {
    /// An error occurred while checking the layouts of displacements or entries
    LayoutError(LayoutError),
    /// A key is not located at the correct position
    InvalidKeyPosition {
        /// The index of the key when iterating
        index: usize,
    },
}

impl fmt::Display for HashMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashMapError::LayoutError(e) => write!(f, "layout error: {}", e),
            HashMapError::InvalidKeyPosition { index } => {
                write!(f, "invalid key position: at index {}", index)
            }
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for HashMapError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                HashMapError::HashIndexError(e) => Some(e as &dyn Error),
                HashMapError::LayoutError(e) => Some(e as &dyn Error),
                HashMapError::CheckEntryError(e) => Some(e as &dyn Error),
                HashMapError::InvalidKeyPosition { .. } => None,
                HashMapError::ContextError(e) => Some(e as &dyn Error),
            }
        }
    }
};

unsafe impl<K, V, C, E> CheckBytes<C, E> for ArchivedHashMap<K, V>
where
    K: CheckBytes<C, E> + Eq + Hash,
    V: CheckBytes<C, E>,
    C: ArchiveContext<E> + ?Sized,
{
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut C,
    ) -> Result<(), E> {
        let index = ArchivedHashIndex::check_bytes(
            ptr::addr_of!((*value).index),
            context,
        )?;
        Layout::array::<Entry<K, V>>(index.len())?;

        let entries_rel_ptr = RelPtr::manual_check_bytes(
            ptr::addr_of!((*value).entries),
            context,
        )?;
        let entries_ptr = context
            .check_subtree_ptr::<[Entry<K, V>]>(
                entries_rel_ptr.base(),
                entries_rel_ptr.offset(),
                index.len(),
            )
            .map_err(HashMapError::ContextError)?;

        let range = context
            .push_prefix_subtree(entries_ptr)
            .map_err(HashMapError::ContextError)?;
        let entries = <[Entry<K, V>]>::check_bytes(entries_ptr, context)?;
        context
            .pop_prefix_range(range)
            .map_err(HashMapError::ContextError)?;

        for (i, entry) in entries.iter().enumerate() {
            if index.index(&entry.key) != Some(i) {
                return Err(HashMapError::InvalidKeyPosition { index: i });
            }
        }

        Ok(&*value)
    }
}
