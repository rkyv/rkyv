//! Archived index set implementation.
//!
//! During archiving, index sets are built into minimal perfect index sets using
//! [compress, hash and displace](http://cmph.sourceforge.net/papers/esa09.pdf).

use core::{borrow::Borrow, fmt, hash::{Hash, Hasher}};

use rancor::{Error, Fallible};

use crate::{
    collections::swiss_table::{
        index_map::Keys, ArchivedIndexMap, IndexMapResolver,
    }, hash::FxHasher64, out_field, ser::{Allocator, Writer}, Serialize
};

/// An archived `IndexSet`.
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedIndexSet<K, H = FxHasher64> {
    inner: ArchivedIndexMap<K, (), H>,
}

impl<K, H> ArchivedIndexSet<K, H> {
    /// Returns whether the index set contains no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the keys of the index set in order.
    #[inline]
    pub fn iter(&self) -> Keys<K, ()> {
        self.inner.keys()
    }

    /// Returns the number of elements in the index set.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K, H: Default + Hasher> ArchivedIndexSet<K, H> {
    /// Returns whether a key is present in the hash set.
    #[inline]
    pub fn contains<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.contains_key(k)
    }

    /// Returns the value stored in the set, if any.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.get_full(k).map(|(_, k, _)| k)
    }

    /// Returns the item index and value stored in the set, if any.
    #[inline]
    pub fn get_full<Q: ?Sized>(&self, k: &Q) -> Option<(usize, &K)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.get_full(k).map(|(i, k, _)| (i, k))
    }

    /// Gets a key by index.
    #[inline]
    pub fn get_index(&self, index: usize) -> Option<&K> {
        self.inner.get_index(index).map(|(k, _)| k)
    }

    /// Returns the index of a key if it exists in the set.
    #[inline]
    pub fn get_index_of<Q: ?Sized>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.get_index_of(key)
    }

    /// Resolves an archived index map from a given length and parameters.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that properly aligned and valid for writes.
    #[inline]
    pub unsafe fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        pos: usize,
        resolver: IndexSetResolver,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.inner);
        ArchivedIndexMap::resolve_from_len(
            len,
            load_factor,
            pos + fp,
            resolver.0,
            fo,
        );
    }

    /// Serializes an iterator of keys as an index set.
    #[inline]
    pub fn serialize_from_iter<'a, I, UK, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<IndexSetResolver, S::Error>
    where
        I: Clone + ExactSizeIterator<Item = &'a UK>,
        UK: 'a + Serialize<S, Archived = K> + Hash + Eq,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Error,
    {
        Ok(IndexSetResolver(ArchivedIndexMap::<K, (), H>::serialize_from_iter(
            iter.map(|x| (x, &())),
            load_factor,
            serializer,
        )?))
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
