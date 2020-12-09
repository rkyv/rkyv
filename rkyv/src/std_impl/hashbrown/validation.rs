//! Validation implementations for HashMap and HashSet.

use super::{ArchivedBucket, ArchivedHashMap, ArchivedHashSet, Group};
use crate::{
    offset_of,
    validation::{ArchiveContext, ArchiveMemoryError},
    RelPtr,
};
use bytecheck::{CheckBytes, Unreachable};
use core::{fmt, hash::Hash};
use std::error::Error;

/// Errors that can occur while checking an archived bucket.
#[derive(Debug)]
pub enum ArchivedBucketError<K, V> {
    /// An error occurred while checking the bytes of a key
    KeyCheckBytes(K),
    /// An error occurred while checking the bytes of a value
    ValueCheckBytes(V),
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for ArchivedBucketError<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedBucketError::KeyCheckBytes(e) => write!(f, "key check error: {}", e),
            ArchivedBucketError::ValueCheckBytes(e) => write!(f, "value check error: {}", e),
        }
    }
}

impl<K: fmt::Debug + fmt::Display, V: fmt::Debug + fmt::Display> Error
    for ArchivedBucketError<K, V>
{
}

impl<K: CheckBytes<ArchiveContext>, V: CheckBytes<ArchiveContext>> CheckBytes<ArchiveContext>
    for ArchivedBucket<K, V>
{
    type Error = ArchivedBucketError<K::Error, V::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        K::check_bytes(bytes.add(offset_of!(ArchivedBucket<K, V>, key)), context)
            .map_err(ArchivedBucketError::KeyCheckBytes)?;
        V::check_bytes(bytes.add(offset_of!(ArchivedBucket<K, V>, value)), context)
            .map_err(ArchivedBucketError::ValueCheckBytes)?;
        Ok(&*bytes.cast())
    }
}

/// Errors that can occur while checking an [`ArchivedHashMap`].
#[derive(Debug)]
pub enum ArchivedHashMapError<K, V> {
    /// The number of items the hashmap claims to have doesn't match the actual
    /// number of items as indicated by the control bytes.
    InvalidItemCount {
        /// The number of items the hashmap claims to have
        expected_items: usize,
        /// The actual number of items in the hashmap
        actual_items: usize,
    },
    /// A memory error occurred
    MemoryError(ArchiveMemoryError),
    /// An error occured while checking the bytes of a bucket
    BucketError(ArchivedBucketError<K, V>),
    /// A key is placed in the wrong bucket
    IncorrectKeyHash {
        /// The index of the key when iterating
        index: usize,
    },
}

impl<K: fmt::Display, V: fmt::Display> fmt::Display for ArchivedHashMapError<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedHashMapError::InvalidItemCount {
                expected_items,
                actual_items,
            } => write!(
                f,
                "invalid item count: expected {} items, found {} items",
                expected_items, actual_items
            ),
            ArchivedHashMapError::MemoryError(e) => write!(f, "hash map memory error: {}", e),
            ArchivedHashMapError::BucketError(e) => write!(f, "hash map bucket error: {}", e),
            ArchivedHashMapError::IncorrectKeyHash { index } => {
                write!(f, "incorrect key hash: at index {}", index)
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

impl<K, V> From<ArchiveMemoryError> for ArchivedHashMapError<K, V> {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl<K, V> From<ArchivedBucketError<K, V>> for ArchivedHashMapError<K, V> {
    fn from(e: ArchivedBucketError<K, V>) -> Self {
        Self::BucketError(e)
    }
}

impl<K: CheckBytes<ArchiveContext> + Eq + Hash, V: CheckBytes<ArchiveContext>>
    CheckBytes<ArchiveContext> for ArchivedHashMap<K, V>
{
    type Error = ArchivedHashMapError<K::Error, V::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        let bucket_mask = *u32::check_bytes(
            bytes.add(offset_of!(ArchivedHashMap<K, V>, bucket_mask)),
            context,
        )?;
        let buckets = bucket_mask as usize + 1;

        let ctrl_ptr =
            RelPtr::check_bytes(bytes.add(offset_of!(ArchivedHashMap<K, V>, ctrl)), context)?;
        let ctrl = context.claim_bytes(
            (ctrl_ptr as *const RelPtr).cast(),
            ctrl_ptr.offset(),
            buckets + Group::WIDTH,
            Group::WIDTH,
        )?;

        let data_ptr =
            RelPtr::check_bytes(bytes.add(offset_of!(ArchivedHashMap<K, V>, data)), context)?;
        let data = context.claim::<ArchivedBucket<K, V>>(
            (data_ptr as *const RelPtr).cast(),
            data_ptr.offset(),
            buckets,
        )?;

        let items = *u32::check_bytes(bytes.add(offset_of!(ArchivedHashMap<K, V>, items)), context)?
            as usize;

        let mut actual_items = 0;

        // Iterate through control bytes and check full buckets
        let mut current_group = Group::load_aligned(ctrl).match_full();
        let mut current_data = data.cast::<ArchivedBucket<K, V>>();
        let mut next_ctrl = ctrl.add(Group::WIDTH);
        let end = ctrl.add(buckets);

        loop {
            let next = loop {
                if let Some(index) = current_group.lowest_set_bit() {
                    current_group = current_group.remove_lowest_bit();
                    break Some(current_data.add(index));
                }

                if next_ctrl >= end {
                    break None;
                }

                current_group = Group::load_aligned(next_ctrl).match_full();
                current_data = current_data.add(Group::WIDTH);
                next_ctrl = next_ctrl.add(Group::WIDTH);
            };

            if let Some(bucket) = next {
                actual_items += 1;
                ArchivedBucket::<K, V>::check_bytes(bucket.cast(), context)?;
            } else {
                break;
            }
        }

        if items != actual_items {
            return Err(ArchivedHashMapError::InvalidItemCount {
                expected_items: items,
                actual_items,
            });
        }

        // At this point, everything checks out and we just need to make sure
        // that the keys point to their own position. That's something we can do
        // by just iterating through the hash map.
        let hash_map = &*bytes.cast::<ArchivedHashMap<K, V>>();
        for (index, key) in hash_map.keys().enumerate() {
            if !hash_map.contains_key(key) {
                return Err(ArchivedHashMapError::IncorrectKeyHash { index });
            }
        }

        Ok(hash_map)
    }
}

impl<K: CheckBytes<ArchiveContext> + Eq + Hash> CheckBytes<ArchiveContext> for ArchivedHashSet<K> {
    type Error = ArchivedHashMapError<K::Error, <() as CheckBytes<ArchiveContext>>::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        ArchivedHashMap::<K, ()>::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}
