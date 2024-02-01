//! Archived hash set implementation.
//!
//! During archiving, hashsets are built into minimal perfect hashsets using
//! [compress, hash and displace](http://cmph.sourceforge.net/papers/esa09.pdf).

use crate::collections::swiss_table::{
    ArchivedSwissTable, Keys, SwissTableResolver,
};
#[cfg(feature = "alloc")]
use crate::{
    ser::{Allocator, Writer},
    Serialize,
};
use core::{borrow::Borrow, fmt, hash::Hash};
use rancor::{Error, Fallible};

/// An archived `HashSet`. This is a wrapper around a hash map with the same key and a value of
/// `()`.
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedSwissSet<K>(ArchivedSwissTable<K, ()>);

impl<K> ArchivedSwissSet<K> {
    /// Gets the number of items in the hash set.
    #[inline]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// Gets the key corresponding to the given key in the hash set.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&K>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.get_key_value(k).map(|(k, _)| k)
    }

    /// Returns whether the given key is in the hash set.
    #[inline]
    pub fn contains<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.0.contains_key(k)
    }

    /// Returns whether there are no items in the hash set.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets an iterator over the keys of the underlying hash map.
    #[inline]
    pub fn iter(&self) -> Keys<K, ()> {
        self.0.keys()
    }

    /// Resolves an archived hash set from the given length and parameters.
    ///
    /// # Safety
    ///
    /// - `len` must be the number of elements that were serialized
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing a hash map
    #[inline]
    pub unsafe fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        pos: usize,
        resolver: SwissSetResolver,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.0);
        ArchivedSwissTable::resolve_from_len(
            len,
            load_factor,
            pos + fp,
            resolver.0,
            fo,
        );
    }

    /// Serializes an iterator of keys as a hash set.
    #[cfg(feature = "alloc")]
    #[inline]
    pub fn serialize_from_iter<'a, KU, S, I>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<SwissSetResolver, S::Error>
    where
        KU: 'a + Serialize<S, Archived = K> + Hash + Eq,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Error,
        I: Clone + ExactSizeIterator<Item = &'a KU>,
    {
        Ok(SwissSetResolver(ArchivedSwissTable::serialize_from_iter(
            iter.map(|x| (x, &())),
            load_factor,
            serializer,
        )?))
    }
}

impl<K: fmt::Debug> fmt::Debug for ArchivedSwissSet<K> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.iter()).finish()
    }
}

/// The resolver for archived hash sets.
pub struct SwissSetResolver(SwissTableResolver);

impl<K: Hash + Eq> PartialEq for ArchivedSwissSet<K> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K: Hash + Eq> Eq for ArchivedSwissSet<K> {}
