//! Archived hash map implementation using an archived SwissTable.

use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::Index,
    pin::Pin,
};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    collections::{
        swiss_table::table::{ArchivedHashTable, HashTableResolver, RawIter},
        util::{Entry, EntryAdapter},
    },
    hash::{hash_value, FxHasher64},
    ser::{Allocator, Writer},
    Place, Portable, Serialize,
};

/// An archived SwissTable hash map.
#[derive(Portable)]
#[archive(crate)]
#[repr(transparent)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
pub struct ArchivedHashMap<K, V, H = FxHasher64> {
    table: ArchivedHashTable<Entry<K, V>>,
    _phantom: PhantomData<H>,
}

impl<K, V, H> ArchivedHashMap<K, V, H> {
    /// Returns whether the hash map is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of elements in the hash map.
    #[inline]
    pub const fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns the total capacity of the hash map.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// Returns an iterator over the key-value entries in the hash map.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V, H> {
        Iter {
            raw: self.table.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the mutable key-value entries in the hash map.
    #[inline]
    pub fn iter_mut(self: Pin<&mut Self>) -> IterMut<'_, K, V, H> {
        let table = unsafe { self.map_unchecked_mut(|s| &mut s.table) };
        IterMut {
            raw: table.raw_iter_mut(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the keys in the hash map.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V, H> {
        Keys {
            raw: self.table.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the values in the hash map.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V, H> {
        Values {
            raw: self.table.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the mutable values in the hash map.
    #[inline]
    pub fn values_mut(self: Pin<&mut Self>) -> ValuesMut<'_, K, V, H> {
        let table = unsafe { self.map_unchecked_mut(|s| &mut s.table) };
        ValuesMut {
            raw: table.raw_iter_mut(),
            _phantom: PhantomData,
        }
    }
}

impl<K, V, H: Hasher + Default> ArchivedHashMap<K, V, H> {
    /// Returns the key-value pair corresponding to the supplied key using the
    /// given comparison function.
    #[inline]
    pub fn get_key_value_with<Q, C>(&self, key: &Q, cmp: C) -> Option<(&K, &V)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let entry = self
            .table
            .get_with(hash_value::<Q, H>(key), |e| cmp(key, &e.key))?;
        Some((&entry.key, &entry.value))
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_key_value_with(key, |q, k| q == k.borrow())
    }

    /// Returns a reference to the value corresponding to the supplied key using
    /// the given comparison function.
    #[inline]
    pub fn get_with<Q, C>(&self, key: &Q, cmp: C) -> Option<&V>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        Some(self.get_key_value_with(key, cmp)?.1)
    }

    /// Returns a reference to the value corresponding to the supplied key.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_key_value(key)?.1)
    }

    /// Returns the mutable key-value pair corresponding to the supplied key
    /// using the given comparison function.
    #[inline]
    pub fn get_key_value_mut_with<Q, C>(
        self: Pin<&mut Self>,
        key: &Q,
        cmp: C,
    ) -> Option<(&K, Pin<&mut V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let table = unsafe { Pin::map_unchecked_mut(self, |s| &mut s.table) };
        let entry = table
            .get_with_mut(hash_value::<Q, H>(key), |e| cmp(key, &e.key))?;
        let entry = unsafe { Pin::into_inner_unchecked(entry) };
        let key = &entry.key;
        let value = unsafe { Pin::new_unchecked(&mut entry.value) };
        Some((key, value))
    }

    /// Returns the mutable key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value_mut<Q>(
        self: Pin<&mut Self>,
        key: &Q,
    ) -> Option<(&K, Pin<&mut V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_key_value_mut_with(key, |q, k| q == k.borrow())
    }

    /// Returns a mutable reference to the value corresponding to the supplied
    /// key using the given comparison function.
    #[inline]
    pub fn get_mut_with<Q, C>(
        self: Pin<&mut Self>,
        key: &Q,
        cmp: C,
    ) -> Option<Pin<&mut V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        Some(self.get_key_value_mut_with(key, cmp)?.1)
    }

    /// Returns a mutable reference to the value corresponding to the supplied
    /// key.
    #[inline]
    pub fn get_mut<Q>(self: Pin<&mut Self>, key: &Q) -> Option<Pin<&mut V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_key_value_mut(key)?.1)
    }

    /// Returns whether the hash map contains the given key.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Serializes an iterator of key-value pairs as a hash map.
    pub fn serialize_from_iter<'a, I, KU, VU, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<HashMapResolver, S::Error>
    where
        I: Clone + ExactSizeIterator<Item = (&'a KU, &'a VU)>,
        KU: 'a + Serialize<S, Archived = K> + Hash + Eq,
        VU: 'a + Serialize<S, Archived = V>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        ArchivedHashTable::<Entry<K, V>>::serialize_from_iter(
            iter.clone().map(|(key, value)| EntryAdapter { key, value }),
            iter.map(|(key, _)| hash_value::<KU, H>(key)),
            load_factor,
            serializer,
        )
        .map(HashMapResolver)
    }

    /// Resolves an archived hash map from a given length and parameters.
    pub fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        resolver: HashMapResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedHashMap { table, _phantom: _ } = out);
        ArchivedHashTable::<Entry<K, V>>::resolve_from_len(
            len,
            load_factor,
            resolver.0,
            table,
        )
    }
}

impl<K, V, H> fmt::Debug for ArchivedHashMap<K, V, H>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V, H> Eq for ArchivedHashMap<K, V, H>
where
    K: Hash + Eq,
    V: Eq,
    H: Default + Hasher,
{
}

impl<K, V, H> PartialEq for ArchivedHashMap<K, V, H>
where
    K: Hash + Eq,
    V: PartialEq,
    H: Default + Hasher,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|(key, value)| {
                other.get(key).map_or(false, |v| *value == *v)
            })
        }
    }
}

impl<K, Q, V, H> Index<&'_ Q> for ArchivedHashMap<K, V, H>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
    H: Default + Hasher,
{
    type Output = V;

    #[inline]
    fn index(&self, key: &Q) -> &V {
        self.get(key).unwrap()
    }
}

/// The resolver for [`ArchivedHashMap`].
pub struct HashMapResolver(HashTableResolver);

/// An iterator over the key-value pairs of an [`ArchivedHashMap`].
pub struct Iter<'a, K, V, H> {
    raw: RawIter<Entry<K, V>>,
    _phantom: PhantomData<&'a ArchivedHashMap<K, V, H>>,
}

impl<'a, K, V, H> Iterator for Iter<'a, K, V, H> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { entry.as_ref() };
            (&entry.key, &entry.value)
        })
    }
}

impl<K, V, H> ExactSizeIterator for Iter<'_, K, V, H> {
    #[inline]
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<K, V, H> FusedIterator for Iter<'_, K, V, H> {}

/// An iterator over the mutable key-value pairs of an [`ArchivedHashMap`].
pub struct IterMut<'a, K, V, H> {
    raw: RawIter<Entry<K, V>>,
    _phantom: PhantomData<&'a ArchivedHashMap<K, V, H>>,
}

impl<'a, K, V, H> Iterator for IterMut<'a, K, V, H> {
    type Item = (&'a K, Pin<&'a mut V>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|mut entry| {
            let entry = unsafe { entry.as_mut() };
            let value = unsafe { Pin::new_unchecked(&mut entry.value) };
            (&entry.key, value)
        })
    }
}

impl<K, V, H> ExactSizeIterator for IterMut<'_, K, V, H> {
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<K, V, H> FusedIterator for IterMut<'_, K, V, H> {}

/// An iterator over the keys of an [`ArchivedHashMap`].
pub struct Keys<'a, K, V, H> {
    raw: RawIter<Entry<K, V>>,
    _phantom: PhantomData<&'a ArchivedHashMap<K, V, H>>,
}

impl<'a, K, V, H> Iterator for Keys<'a, K, V, H> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { entry.as_ref() };
            &entry.key
        })
    }
}

impl<K, V, H> ExactSizeIterator for Keys<'_, K, V, H> {
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<K, V, H> FusedIterator for Keys<'_, K, V, H> {}

/// An iterator over the values of an [`ArchivedHashMap`].
pub struct Values<'a, K, V, H> {
    raw: RawIter<Entry<K, V>>,
    _phantom: PhantomData<&'a ArchivedHashMap<K, V, H>>,
}

impl<'a, K, V, H> Iterator for Values<'a, K, V, H> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { entry.as_ref() };
            &entry.value
        })
    }
}

impl<K, V, H> ExactSizeIterator for Values<'_, K, V, H> {
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<K, V, H> FusedIterator for Values<'_, K, V, H> {}

/// An iterator over the mutable values of an [`ArchivedHashMap`].
pub struct ValuesMut<'a, K, V, H> {
    raw: RawIter<Entry<K, V>>,
    _phantom: PhantomData<&'a ArchivedHashMap<K, V, H>>,
}

impl<'a, K, V, H> Iterator for ValuesMut<'a, K, V, H> {
    type Item = Pin<&'a mut V>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|mut entry| {
            let entry = unsafe { entry.as_mut() };
            unsafe { Pin::new_unchecked(&mut entry.value) }
        })
    }
}

impl<K, V, H> ExactSizeIterator for ValuesMut<'_, K, V, H> {
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<K, V, H> FusedIterator for ValuesMut<'_, K, V, H> {}
