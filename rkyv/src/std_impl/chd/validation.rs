//! Validation implementations for HashMap and HashSet.

use super::{ArchivedHashMap, ArchivedHashSet, Entry};
use crate::{
    offset_of,
    ArchiveContext,
    ArchiveMemoryError,
    RelPtr,
};
use bytecheck::{CheckBytes, Unreachable};
use core::{
    fmt,
    hash::{Hash, Hasher},
    slice,
};
use std::error::Error;

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

impl<K: fmt::Debug + fmt::Display, V: fmt::Debug + fmt::Display> Error
    for ArchivedHashMapEntryError<K, V>
{
}

impl<K: CheckBytes<C>, V: CheckBytes<C>, C: ArchiveContext + ?Sized> CheckBytes<C>
    for Entry<K, V>
{
    type Error = ArchivedHashMapEntryError<K::Error, V::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        K::check_bytes(bytes.add(offset_of!(Entry<K, V>, key)), context)
            .map_err(ArchivedHashMapEntryError::KeyCheckError)?;
        V::check_bytes(bytes.add(offset_of!(Entry<K, V>, value)), context)
            .map_err(ArchivedHashMapEntryError::ValueCheckError)?;
        Ok(&*bytes.cast())
    }
}

/// Errors that can occur while checking an archived hash map.
#[derive(Debug)]
pub enum ArchivedHashMapError<K, V> {
    /// An error occured while checking the bytes of an entry
    CheckEntryError(ArchivedHashMapEntryError<K, V>),
    /// A memory error occurred
    MemoryError(ArchiveMemoryError),
    /// A displacement value was invalid
    InvalidDisplacement { index: usize, value: u32 },
    /// A key is not located at the correct position
    InvalidKeyPosition {
        /// The index of the key when iterating
        index: usize,
    },
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for ArchivedHashMapError<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedHashMapError::CheckEntryError(e) => write!(f, "entry check error: {}", e),
            ArchivedHashMapError::MemoryError(e) => write!(f, "hash map memory error: {}", e),
            ArchivedHashMapError::InvalidDisplacement { index, value } => write!(
                f,
                "invalid displacement: value {} at index {}",
                value, index
            ),
            ArchivedHashMapError::InvalidKeyPosition { index } => {
                write!(f, "invalid key position: at index {}", index)
            }
        }
    }
}

impl<K: fmt::Debug + fmt::Display, V: fmt::Debug + fmt::Display> Error
    for ArchivedHashMapError<K, V>
{
}

impl<K, V> From<Unreachable> for ArchivedHashMapError<K, V> {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<K, V> From<ArchivedHashMapEntryError<K, V>> for ArchivedHashMapError<K, V> {
    fn from(e: ArchivedHashMapEntryError<K, V>) -> Self {
        Self::CheckEntryError(e)
    }
}

impl<K, V> From<ArchiveMemoryError> for ArchivedHashMapError<K, V> {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl<K: CheckBytes<C> + Eq + Hash, V: CheckBytes<C>, C: ArchiveContext + ?Sized>
    CheckBytes<C> for ArchivedHashMap<K, V>
{
    type Error = ArchivedHashMapError<K::Error, V::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let len = *u32::check_bytes(bytes.add(offset_of!(ArchivedHashMap<K, V>, len)), context)?;

        let displace_ptr = RelPtr::check_bytes(
            bytes.add(offset_of!(ArchivedHashMap<K, V>, displace)),
            context,
        )?;
        let displace = context
            .claim::<u32>(displace_ptr, len as usize)?
            .cast::<u32>();

        let displace = slice::from_raw_parts(displace, len as usize);
        for (i, &d) in displace.iter().enumerate() {
            if d >= len && d < 0x80_00_00_00 {
                return Err(ArchivedHashMapError::InvalidDisplacement { index: i, value: d });
            }
        }

        let entries_ptr = RelPtr::check_bytes(
            bytes.add(offset_of!(ArchivedHashMap<K, V>, entries)),
            context,
        )?;
        let entries = context
            .claim::<Entry<K, V>>(entries_ptr, len as usize)?
            .cast::<Entry<K, V>>();

        for i in 0..len {
            let entry = Entry::<K, V>::check_bytes(entries.add(i as usize).cast::<u8>(), context)?;

            let mut hasher = ArchivedHashMap::<K, V>::make_hasher();
            entry.key.hash(&mut hasher);
            let displace_index = hasher.finish() % len as u64;
            let displace = displace[displace_index as usize];

            let index = if displace == u32::MAX {
                return Err(ArchivedHashMapError::InvalidKeyPosition { index: i as usize });
            } else if displace & 0x80_00_00_00 == 0 {
                displace as u64
            } else {
                let mut hasher = ArchivedHashMap::<K, V>::make_hasher();
                displace.hash(&mut hasher);
                entry.key.hash(&mut hasher);
                hasher.finish() % len as u64
            };

            if index != i as u64 {
                return Err(ArchivedHashMapError::InvalidKeyPosition { index: i as usize });
            }
        }

        Ok(&*bytes.cast())
    }
}

impl<K: CheckBytes<C> + Hash + Eq, C: ArchiveContext + ?Sized> CheckBytes<C> for ArchivedHashSet<K> {
    type Error = ArchivedHashMapError<K::Error, <() as CheckBytes<C>>::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        ArchivedHashMap::<K, ()>::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}
