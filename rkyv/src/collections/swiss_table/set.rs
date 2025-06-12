//! Archived hash set implementation using an archived SwissTable.

use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::map::{ArchivedHashMap, HashMapResolver, Keys},
    hash::FxHasher64,
    ser::{Allocator, Writer},
    Place, Portable, Serialize,
};

/// An archived `HashSet`. This is a wrapper around a hash map with the same key
/// and unit value.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedHashSet<K, H = FxHasher64> {
    inner: ArchivedHashMap<K, (), H>,
}

impl<K, H> ArchivedHashSet<K, H> {
    /// Gets the number of items in the hash set.
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are no items in the hash set.
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Gets an iterator over the keys of the underlying hash map.
    pub fn iter(&self) -> Keys<'_, K, (), H> {
        self.inner.keys()
    }
}

impl<K, H: Hasher + Default> ArchivedHashSet<K, H> {
    /// Gets the key corresponding to the given key in the hash set.
    pub fn get<Q>(&self, k: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_key_value(k).map(|(k, _)| k)
    }

    /// Returns whether the given key is in the hash set.
    pub fn contains<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(k)
    }

    /// Resolves an archived hash set from the given length and parameters.
    pub fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        resolver: HashSetResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedHashSet { inner } = out);
        ArchivedHashMap::resolve_from_len(len, load_factor, resolver.0, inner);
    }

    /// Serializes an iterator of keys as a hash set.
    pub fn serialize_from_iter<I, KU, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<HashSetResolver, S::Error>
    where
        I: Clone + ExactSizeIterator,
        I::Item: Borrow<KU>,
        KU: Serialize<S, Archived = K> + Hash + Eq,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        Ok(HashSetResolver(
            ArchivedHashMap::<K, (), H>::serialize_from_iter::<
                _,
                _,
                (),
                _,
                _,
                _,
            >(iter.map(|x| (x, ())), load_factor, serializer)?,
        ))
    }
}

impl<K: fmt::Debug, H> fmt::Debug for ArchivedHashSet<K, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<K: Hash + Eq, H: Hasher + Default> PartialEq for ArchivedHashSet<K, H> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<K: Hash + Eq, H: Hasher + Default> Eq for ArchivedHashSet<K, H> {}

/// The resolver for archived hash sets.
pub struct HashSetResolver(HashMapResolver);
