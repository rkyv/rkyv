//! Validation implementation for ArchivedIndexMap.

use crate::{
    collections::{
        index_map::ArchivedIndexMap,
        util::Entry,
        ArchivedHashIndex,
    },
    primitive::ArchivedUsize,
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

/// Errors that can occur while checking an archived index map.
#[derive(Debug)]
pub enum IndexMapError {
    /// An error occurred while checking the layouts of displacements or entries
    LayoutError(LayoutError),
    /// A pivot indexes outside of the entries array
    PivotOutOfBounds {
        /// The index of the pivot when iterating
        index: usize,
        /// The pivot value that was invalid
        pivot: usize,
    },
    /// A key is not located at the correct position
    ///
    /// This can either be due to the key being invalid for the hash index, or the pivot for the key
    /// not pointing to it.
    InvalidKeyPosition {
        /// The index of the key when iterating
        index: usize,
    },
}

impl fmt::Display for IndexMapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexMapError::LayoutError(e) => write!(f, "layout error: {}", e),
            IndexMapError::PivotOutOfBounds { index, pivot } => {
                write!(f, "pivot out of bounds: {} at index {}", pivot, index)
            }
            IndexMapError::InvalidKeyPosition { index } => {
                write!(f, "invalid key position: at index {}", index)
            }
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for IndexMapError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                IndexMapError::LayoutError(e) => Some(e as &dyn Error),
                IndexMapError::PivotOutOfBounds { .. } => None,
                IndexMapError::InvalidKeyPosition { .. } => None,
            }
        }
    }
};

unsafe impl<K, V, C, E> CheckBytes<C, E> for ArchivedIndexMap<K, V>
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

        // Entries
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
            .map_err(IndexMapError::ContextError)?;

        let range = context
            .push_prefix_subtree(entries_ptr)
            .map_err(IndexMapError::ContextError)?;
        let entries = <[Entry<K, V>]>::check_bytes(entries_ptr, context)?;
        context
            .pop_prefix_range(range)
            .map_err(IndexMapError::ContextError)?;

        // Pivots
        Layout::array::<ArchivedUsize>(index.len())?;
        let pivots_rel_ptr = RelPtr::manual_check_bytes(
            ptr::addr_of!((*value).pivots),
            context,
        )?;
        let pivots_ptr = context
            .check_subtree_ptr::<[ArchivedUsize]>(
                pivots_rel_ptr.base(),
                pivots_rel_ptr.offset(),
                index.len(),
            )
            .map_err(IndexMapError::ContextError)?;

        let range = context
            .push_prefix_subtree(pivots_ptr)
            .map_err(IndexMapError::ContextError)?;
        let pivots = <[ArchivedUsize]>::check_bytes(pivots_ptr, context)?;
        context
            .pop_prefix_range(range)
            .map_err(IndexMapError::ContextError)?;

        for (i, pivot) in pivots.iter().enumerate() {
            let pivot = pivot.to_native() as usize;
            if pivot >= index.len() {
                return Err(IndexMapError::PivotOutOfBounds {
                    index: i,
                    pivot,
                });
            }
        }

        for (i, entry) in entries.iter().enumerate() {
            if let Some(pivot_index) = index.index(&entry.key) {
                let pivot = pivots[pivot_index].to_native() as usize;
                if pivot != i {
                    return Err(IndexMapError::InvalidKeyPosition { index: i });
                }
            } else {
                return Err(IndexMapError::InvalidKeyPosition { index: i });
            }
        }

        Ok(&*value)
    }
}
