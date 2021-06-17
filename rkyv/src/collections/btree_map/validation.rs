//! Validation implementation for BTreeMap.

use crate::{
    collections::ArchivedBTreeMap,
    validation::ArchiveMemoryContext,
};
use core::fmt;
use bytecheck::CheckBytes;

/// Errors that can occur while checking an archived B-tree.
#[derive(Debug)]
pub enum ArchivedBTreeMapError<K, V> {
    /// An error occurred while checking the bytes of a key
    KeyCheckError(K),
    /// An error occurred while checking the bytes of a value
    ValueCheckError(V),
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for ArchivedBTreeMapError<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedBTreeMapError::KeyCheckError(e) => write!(f, "key check error: {}", e),
            ArchivedBTreeMapError::ValueCheckError(e) => write!(f, "value check error: {}", e),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl<K: Error + 'static, V: Error + 'static> Error for ArchivedBTreeMapError<K, V> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                ArchivedBTreeMapError::KeyCheckError(e) => Some(e as &dyn Error),
                ArchivedBTreeMapError::ValueCheckError(e) => Some(e as &dyn Error),
            }
        }
    }
};

impl<K, V, C> CheckBytes<C> for ArchivedBTreeMap<K, V>
where
    K: CheckBytes<C>,
    V: CheckBytes<C>,
    C: ArchiveMemoryContext + ?Sized,
{
    type Error = ArchivedBTreeMapError<K::Error, V::Error>;

    unsafe fn check_bytes<'a>(_value: *const Self, _context: &mut C) -> Result<&'a Self, Self::Error> {
        todo!();
    }
}
