//! Archived hash set implementation using an archived SwissTable.

use core::hash::Hasher;
use core::{borrow::Borrow, fmt, hash::Hash};

use rancor::{Error, Fallible};

use crate::collections::swiss_table::map::{
    ArchivedHashMap, HashMapResolver, Keys,
};
use crate::hash::FxHasher64;
use crate::{
    ser::{Allocator, Writer},
    Serialize,
};

/// An archived `HashSet`. This is a wrapper around a hash map with the same key
/// and unit value.
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedHashSet<K, H = FxHasher64> {
    inner: ArchivedHashMap<K, (), H>,
}

impl<K, H> ArchivedHashSet<K, H> {
    /// Gets the number of items in the hash set.
    #[inline]
    pub const fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether there are no items in the hash set.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Gets an iterator over the keys of the underlying hash map.
    #[inline]
    pub fn iter(&self) -> Keys<K, (), H> {
        self.inner.keys()
    }
}

impl<K, H: Hasher + Default> ArchivedHashSet<K, H> {
    /// Gets the key corresponding to the given key in the hash set.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.get_key_value(k).map(|(k, _)| k)
    }

    /// Returns whether the given key is in the hash set.
    #[inline]
    pub fn contains<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.inner.contains_key(k)
    }

    /// Resolves an archived hash set from the given length and parameters.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that properly aligned and valid for writes.
    #[inline]
    pub unsafe fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        pos: usize,
        resolver: HashSetResolver,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.inner);
        ArchivedHashMap::resolve_from_len(
            len,
            load_factor,
            pos + fp,
            resolver.0,
            fo,
        );
    }

    /// Serializes an iterator of keys as a hash set.
    #[inline]
    pub fn serialize_from_iter<'a, KU, S, I>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<HashSetResolver, S::Error>
    where
        KU: 'a + Serialize<S, Archived = K> + Hash + Eq,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Error,
        I: Clone + ExactSizeIterator<Item = &'a KU>,
    {
        Ok(HashSetResolver(ArchivedHashMap::<K, (), H>::serialize_from_iter(
            iter.map(|x| (x, &())),
            load_factor,
            serializer,
        )?))
    }
}

impl<K: fmt::Debug, H> fmt::Debug for ArchivedHashSet<K, H> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

impl<K: Hash + Eq, H: Hasher + Default> PartialEq for ArchivedHashSet<K, H> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<K: Hash + Eq, H: Hasher + Default> Eq for ArchivedHashSet<K, H> {}

/// The resolver for archived hash sets.
pub struct HashSetResolver(HashMapResolver);
