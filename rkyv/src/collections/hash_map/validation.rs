//! Validation implementation for HashMap.

use crate::{
    collections::hash_map::{ArchivedHashMap, Entry},
    validation::{ArchiveBoundsContext, ArchiveMemoryContext},
    Archived, RelPtr,
};
use bytecheck::{CheckBytes, Error, SliceCheckError};
use core::{
    alloc::{Layout, LayoutError},
    convert::Infallible,
    fmt,
    hash::{Hash, Hasher},
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

    impl<K: Error + 'static, V: Error + 'static> Error for ArchivedHashMapEntryError<K, V>
    {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                ArchivedHashMapEntryError::KeyCheckError(e) => Some(e as &dyn Error),
                ArchivedHashMapEntryError::ValueCheckError(e) => Some(e as &dyn Error),
            }
        }
    }
};

impl<K: CheckBytes<C>, V: CheckBytes<C>, C: ArchiveMemoryContext + ?Sized> CheckBytes<C>
    for Entry<K, V>
{
    type Error = ArchivedHashMapEntryError<K::Error, V::Error>;

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
    /// An error occured while checking the layouts of displacements or entries
    LayoutError(LayoutError),
    /// An error occured while checking the entries
    CheckEntryError(SliceCheckError<ArchivedHashMapEntryError<K, V>>),
    /// A displacement value was invalid
    InvalidDisplacement {
        /// The index of the entry with an invalid displacement
        index: usize,
        /// The value of the entry at the invalid location
        value: u32,
    },
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
            HashMapError::CheckEntryError(e) => write!(f, "entry check error: {}", e),
            HashMapError::InvalidDisplacement { index, value } => write!(
                f,
                "invalid displacement: value {} at index {}",
                value, index
            ),
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
                HashMapError::LayoutError(e) => Some(e as &dyn Error),
                HashMapError::CheckEntryError(e) => Some(e as &dyn Error),
                HashMapError::InvalidDisplacement { .. } => None,
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
    C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized,
    C::Error: Error,
{
    type Error = HashMapError<K::Error, V::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let len = from_archived!(*Archived::<usize>::check_bytes(
            ptr::addr_of!((*value).len),
            context,
        )?) as usize;

        let displace_rel_ptr =
            RelPtr::manual_check_bytes(ptr::addr_of!((*value).displace), context)?;
        let displace_data_ptr = context
            .check_rel_ptr(displace_rel_ptr.base(), displace_rel_ptr.offset())
            .map_err(HashMapError::ContextError)?;
        Layout::array::<Archived<u32>>(len)?;
        let displace_ptr = ptr_meta::from_raw_parts(displace_data_ptr.cast(), len);
        context
            .claim_owned_ptr(displace_ptr)
            .map_err(HashMapError::ContextError)?;
        let displace = <[Archived<u32>]>::check_bytes(displace_ptr, context)?;

        for (i, &d) in displace.iter().enumerate() {
            let d = from_archived!(d);
            if d as usize >= len && d < 0x80_00_00_00 {
                return Err(HashMapError::InvalidDisplacement { index: i, value: d });
            }
        }

        let entries_rel_ptr =
            RelPtr::manual_check_bytes(ptr::addr_of!((*value).entries), context)?;
        let entries_data_ptr = context
            .check_rel_ptr(entries_rel_ptr.base(), entries_rel_ptr.offset())
            .map_err(HashMapError::ContextError)?;
        Layout::array::<Entry<K, V>>(len as usize)?;
        let entries_ptr = ptr_meta::from_raw_parts(entries_data_ptr.cast(), len as usize);
        context
            .claim_owned_ptr(entries_ptr)
            .map_err(HashMapError::ContextError)?;
        let entries = <[Entry<K, V>]>::check_bytes(entries_ptr, context)?;

        for (i, entry) in entries.iter().enumerate() {
            let mut hasher = ArchivedHashMap::<K, V>::make_hasher();
            entry.key.hash(&mut hasher);
            let displace_index = hasher.finish() % len as u64;
            let displace = displace[displace_index as usize];

            let index = if displace == u32::MAX {
                return Err(HashMapError::InvalidKeyPosition { index: i });
            } else if displace & 0x80_00_00_00 == 0 {
                from_archived!(displace) as usize
            } else {
                let mut hasher = ArchivedHashMap::<K, V>::make_hasher();
                displace.hash(&mut hasher);
                entry.key.hash(&mut hasher);
                (hasher.finish() % len as u64) as usize
            };

            if index != i {
                return Err(HashMapError::InvalidKeyPosition { index: i });
            }
        }

        Ok(&*value)
    }
}
