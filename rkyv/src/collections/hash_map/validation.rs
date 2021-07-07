//! Validation implementation for HashMap.

use crate::{
    collections::{
        hash_index::validation::HashIndexError,
        hash_map::{ArchivedHashMap, Entry},
        ArchivedHashIndex,
    },
    validation::ArchiveContext,
    RelPtr,
};
use bytecheck::{CheckBytes, Error, SliceCheckError};
use core::{
    alloc::{Layout, LayoutError},
    convert::Infallible,
    fmt,
    hash::Hash,
    ptr,
};

/// Errors that can occur while checking an archived hash map entry.
#[derive(Debug)]
pub enum ArchivedHashMapEntryError<K, V> {
    /// An error occurred while checking the bytes of a key
    KeyCheckError(K),
    /// An error occurred while checking the bytes of a value
    ValueCheckError(V),
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for ArchivedHashMapEntryError<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedHashMapEntryError::KeyCheckError(e) => write!(f, "key check error: {}", e),
            ArchivedHashMapEntryError::ValueCheckError(e) => write!(f, "value check error: {}", e),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl<K: Error + 'static, V: Error + 'static> Error for ArchivedHashMapEntryError<K, V> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                ArchivedHashMapEntryError::KeyCheckError(e) => Some(e as &dyn Error),
                ArchivedHashMapEntryError::ValueCheckError(e) => Some(e as &dyn Error),
            }
        }
    }
};

impl<K, V, C> CheckBytes<C> for Entry<K, V>
where
    K: CheckBytes<C>,
    V: CheckBytes<C>,
    C: ArchiveContext + ?Sized,
{
    type Error = ArchivedHashMapEntryError<K::Error, V::Error>;

    #[inline]
    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        K::check_bytes(ptr::addr_of!((*value).key), context)
            .map_err(ArchivedHashMapEntryError::KeyCheckError)?;
        V::check_bytes(ptr::addr_of!((*value).value), context)
            .map_err(ArchivedHashMapEntryError::ValueCheckError)?;
        Ok(&*value)
    }
}

/// Errors that can occur while checking an archived hash map.
#[derive(Debug)]
pub enum HashMapError<K, V, C> {
    /// An error occured while checking the hash index
    HashIndexError(HashIndexError<C>),
    /// An error occured while checking the layouts of displacements or entries
    LayoutError(LayoutError),
    /// An error occured while checking the entries
    CheckEntryError(SliceCheckError<ArchivedHashMapEntryError<K, V>>),
    /// A key is not located at the correct position
    InvalidKeyPosition {
        /// The index of the key when iterating
        index: usize,
    },
    /// A bounds error occurred
    ContextError(C),
}

impl<K: fmt::Display, V: fmt::Display, E: fmt::Display> fmt::Display for HashMapError<K, V, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashMapError::HashIndexError(e) => write!(f, "hash index check error: {}", e),
            HashMapError::LayoutError(e) => write!(f, "layout error: {}", e),
            HashMapError::CheckEntryError(e) => write!(f, "entry check error: {}", e),
            HashMapError::InvalidKeyPosition { index } => {
                write!(f, "invalid key position: at index {}", index)
            }
            HashMapError::ContextError(e) => e.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl<K, V, C> Error for HashMapError<K, V, C>
    where
        K: Error + 'static,
        V: Error + 'static,
        C: Error + 'static,
    {
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

impl<K, V, C> From<Infallible> for HashMapError<K, V, C> {
    fn from(_: Infallible) -> Self {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

impl<K, V, C> From<SliceCheckError<Infallible>> for HashMapError<K, V, C> {
    #[inline]
    fn from(_: SliceCheckError<Infallible>) -> Self {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

impl<K, V, C> From<HashIndexError<C>> for HashMapError<K, V, C> {
    #[inline]
    fn from(e: HashIndexError<C>) -> Self {
        Self::HashIndexError(e)
    }
}

impl<K, V, C> From<LayoutError> for HashMapError<K, V, C> {
    #[inline]
    fn from(e: LayoutError) -> Self {
        Self::LayoutError(e)
    }
}

impl<K, V, C> From<SliceCheckError<ArchivedHashMapEntryError<K, V>>> for HashMapError<K, V, C> {
    #[inline]
    fn from(e: SliceCheckError<ArchivedHashMapEntryError<K, V>>) -> Self {
        Self::CheckEntryError(e)
    }
}

impl<K, V, C> CheckBytes<C> for ArchivedHashMap<K, V>
where
    K: CheckBytes<C> + Eq + Hash,
    V: CheckBytes<C>,
    C: ArchiveContext + ?Sized,
    C::Error: Error,
{
    type Error = HashMapError<K::Error, V::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let index = ArchivedHashIndex::check_bytes(ptr::addr_of!((*value).index), context)?;
        Layout::array::<Entry<K, V>>(index.len())?;

        let entries_rel_ptr = RelPtr::manual_check_bytes(ptr::addr_of!((*value).entries), context)?;
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
