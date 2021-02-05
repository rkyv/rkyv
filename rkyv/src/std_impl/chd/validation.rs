//! Validation implementations for HashMap and HashSet.

use crate::{
    offset_of,
    std_impl::chd::{ArchivedHashMap, ArchivedHashSet, Entry},
    validation::{ArchiveBoundsContext, ArchiveMemoryContext, LayoutMetadata},
    Fallible,
    RelPtr,
};
use bytecheck::{CheckBytes, SliceCheckError, Unreachable};
use ptr_meta::metadata;
use core::{
    alloc::Layout,
    fmt,
    hash::{Hash, Hasher},
    ptr,
};
use std::{alloc::LayoutErr, error::Error};

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

impl<K: CheckBytes<C>, V: CheckBytes<C>, C: ArchiveMemoryContext + ?Sized> CheckBytes<C>
    for Entry<K, V>
{
    type Error = ArchivedHashMapEntryError<K::Error, V::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        K::check_bytes(bytes.add(offset_of!(Entry<K, V>, key)).cast(), context)
            .map_err(ArchivedHashMapEntryError::KeyCheckError)?;
        V::check_bytes(bytes.add(offset_of!(Entry<K, V>, value)).cast(), context)
            .map_err(ArchivedHashMapEntryError::ValueCheckError)?;
        Ok(&*value)
    }
}

/// Errors that can occur while checking an archived hash map.
#[derive(Debug)]
pub enum HashMapError<K, V, C> {
    /// An error occured while checking the layouts of displacements or entries
    LayoutError(LayoutErr),
    /// An error occured while checking the displacements
    CheckDisplaceError(SliceCheckError<Unreachable>),
    /// An error occured while checking the entries
    CheckEntryError(SliceCheckError<ArchivedHashMapEntryError<K, V>>),
    /// A displacement value was invalid
    InvalidDisplacement { index: usize, value: u32 },
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
            HashMapError::LayoutError(e) => write!(f, "layout error: {}", e),
            HashMapError::CheckDisplaceError(e) => write!(f, "displacements check error: {}", e),
            HashMapError::CheckEntryError(e) => write!(f, "entry check error: {}", e),
            HashMapError::InvalidDisplacement { index, value } => write!(
                f,
                "invalid displacement: value {} at index {}",
                value, index
            ),
            HashMapError::InvalidKeyPosition { index } => {
                write!(f, "invalid key position: at index {}", index)
            },
            HashMapError::ContextError(e) => e.fmt(f),
        }
    }
}

impl<K: Error + 'static, V: Error + 'static, C: Error + 'static> Error
    for HashMapError<K, V, C>
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            HashMapError::LayoutError(e) => Some(e as &dyn Error),
            HashMapError::CheckDisplaceError(e) => Some(e as &dyn Error),
            HashMapError::CheckEntryError(e) => Some(e as &dyn Error),
            HashMapError::InvalidDisplacement { .. } => None,
            HashMapError::InvalidKeyPosition { .. } => None,
            HashMapError::ContextError(e) => Some(e as &dyn Error),
        }
    }
}

impl<K, V, C> From<Unreachable> for HashMapError<K, V, C> {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<K, V, C> From<LayoutErr> for HashMapError<K, V, C> {
    fn from(e: LayoutErr) -> Self {
        Self::LayoutError(e)
    }
}

impl<K, V, C> From<SliceCheckError<Unreachable>> for HashMapError<K, V, C> {
    fn from(e: SliceCheckError<Unreachable>) -> Self {
        Self::CheckDisplaceError(e)
    }
}

impl<K, V, C> From<SliceCheckError<ArchivedHashMapEntryError<K, V>>> for HashMapError<K, V, C> {
    fn from(e: SliceCheckError<ArchivedHashMapEntryError<K, V>>) -> Self {
        Self::CheckEntryError(e)
    }
}

impl<K: CheckBytes<C> + Eq + Hash, V: CheckBytes<C>, C: ArchiveBoundsContext + ArchiveMemoryContext + Fallible + ?Sized>
    CheckBytes<C> for ArchivedHashMap<K, V>
where
    C::Error: Error,
{
    type Error = HashMapError<K::Error, V::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();

        let len = *u32::check_bytes(bytes.add(offset_of!(ArchivedHashMap<K, V>, len)).cast(), context)?;

        let displace_rel_ptr = RelPtr::<u32>::manual_check_bytes(
            bytes.add(offset_of!(ArchivedHashMap<K, V>, displace)).cast(),
            context,
        )?;
        let displace_data_ptr = context.check_rel_ptr(displace_rel_ptr.base(), displace_rel_ptr.offset())
            .map_err(HashMapError::ContextError)?;
        Layout::array::<u32>(len as usize)?;
        let displace_ptr = ptr::slice_from_raw_parts(displace_data_ptr.cast::<u32>(), len as usize);
        let layout = LayoutMetadata::<[u32]>::layout(metadata(displace_ptr));
        context.bounds_check_ptr(displace_ptr.cast(), &layout)
            .map_err(HashMapError::ContextError)?;
        context.claim_bytes(displace_ptr.cast(), layout.size())
            .map_err(HashMapError::ContextError)?;
        let displace = <[u32]>::check_bytes(displace_ptr, context)?;

        for (i, &d) in displace.iter().enumerate() {
            if d >= len && d < 0x80_00_00_00 {
                return Err(HashMapError::InvalidDisplacement { index: i, value: d });
            }
        }

        let entries_rel_ptr = RelPtr::<Entry<K, V>>::manual_check_bytes(
            bytes.add(offset_of!(ArchivedHashMap<K, V>, entries)).cast(),
            context,
        )?;
        let entries_data_ptr = context.check_rel_ptr(entries_rel_ptr.base(), entries_rel_ptr.offset())
            .map_err(HashMapError::ContextError)?;
        Layout::array::<Entry<K, V>>(len as usize)?;
        let entries_ptr = ptr::slice_from_raw_parts(entries_data_ptr.cast::<Entry<K, V>>(), len as usize);
        let layout = LayoutMetadata::<[Entry<K, V>]>::layout(metadata(entries_ptr));
        context.bounds_check_ptr(entries_ptr.cast(), &layout)
            .map_err(HashMapError::ContextError)?;
        context.claim_bytes(entries_ptr.cast(), layout.size())
            .map_err(HashMapError::ContextError)?;
        let entries = <[Entry<K, V>]>::check_bytes(entries_ptr, context)?;

        for i in 0..len as usize {
            let entry = &entries[i];

            let mut hasher = ArchivedHashMap::<K, V>::make_hasher();
            entry.key.hash(&mut hasher);
            let displace_index = hasher.finish() % len as u64;
            let displace = displace[displace_index as usize];

            let index = if displace == u32::MAX {
                return Err(HashMapError::InvalidKeyPosition { index: i as usize });
            } else if displace & 0x80_00_00_00 == 0 {
                displace as u64
            } else {
                let mut hasher = ArchivedHashMap::<K, V>::make_hasher();
                displace.hash(&mut hasher);
                entry.key.hash(&mut hasher);
                hasher.finish() % len as u64
            };

            if index != i as u64 {
                return Err(HashMapError::InvalidKeyPosition { index: i as usize });
            }
        }

        Ok(&*bytes.cast())
    }
}

impl<K: CheckBytes<C> + Hash + Eq, C: ArchiveBoundsContext + ArchiveMemoryContext + Fallible + ?Sized> CheckBytes<C> for ArchivedHashSet<K>
where
    C::Error: Error,
{
    type Error = HashMapError<K::Error, <() as CheckBytes<C>>::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        ArchivedHashMap::<K, ()>::check_bytes(value.cast(), context)?;
        Ok(&*value)
    }
}
