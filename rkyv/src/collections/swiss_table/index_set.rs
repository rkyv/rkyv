//! An archived index set implementation based on Google's high-performance
//! SwissTable hash map.

use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::{
        index_map::Keys, ArchivedIndexMap, IndexMapResolver,
    },
    hash::FxHasher64,
    ser::{Allocator, Writer},
    Place, Portable, Serialize,
};

/// An archived `IndexSet`.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedIndexSet<K, H = FxHasher64> {
    inner: ArchivedIndexMap<K, (), H>,
}

impl<K, H> ArchivedIndexSet<K, H> {
    /// Returns whether the index set contains no values.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the keys of the index set in order.
    pub fn iter(&self) -> Keys<'_, K, ()> {
        self.inner.keys()
    }

    /// Returns the number of elements in the index set.
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, H: Default + Hasher> ArchivedIndexSet<K, H> {
    /// Returns whether a key is present in the hash set.
    pub fn contains<Q>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains_key(k)
    }

    /// Returns the value stored in the set, if any.
    pub fn get<Q>(&self, k: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_full(k).map(|(_, k, _)| k)
    }

    /// Returns the item index and value stored in the set, if any.
    pub fn get_full<Q>(&self, k: &Q) -> Option<(usize, &K)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_full(k).map(|(i, k, _)| (i, k))
    }

    /// Gets a key by index.
    pub fn get_index(&self, index: usize) -> Option<&K> {
        self.inner.get_index(index).map(|(k, _)| k)
    }

    /// Returns the index of a key if it exists in the set.
    pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_index_of(key)
    }

    /// Resolves an archived index map from a given length and parameters.
    pub fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        resolver: IndexSetResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedIndexSet { inner } = out);
        ArchivedIndexMap::resolve_from_len(len, load_factor, resolver.0, inner);
    }

    /// Serializes an iterator of keys as an index set.
    pub fn serialize_from_iter<I, UK, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<IndexSetResolver, S::Error>
    where
        I: Clone + ExactSizeIterator,
        I::Item: Borrow<UK>,
        UK: Serialize<S, Archived = K> + Hash + Eq,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        Ok(IndexSetResolver(
            ArchivedIndexMap::<K, (), H>::serialize_from_iter::<
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

impl<K: fmt::Debug, H> fmt::Debug for ArchivedIndexSet<K, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<K: PartialEq, H> PartialEq for ArchivedIndexSet<K, H> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<K: Eq, H> Eq for ArchivedIndexSet<K, H> {}

/// The resolver for archived index sets.
pub struct IndexSetResolver(IndexMapResolver);
