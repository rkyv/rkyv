//! Archived hash map implementation using an archived SwissTable.

use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::Index,
};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    collections::{
        swiss_table::table::{ArchivedHashTable, HashTableResolver, RawIter},
        util::{Entry, EntryAdapter},
    },
    hash::{hash_value, FxHasher64},
    seal::Seal,
    ser::{Allocator, Writer},
    Place, Portable, Serialize,
};

/// An archived SwissTable hash map.
#[derive(Portable)]
#[rkyv(crate)]
#[repr(transparent)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
pub struct ArchivedHashMap<K, V, H = FxHasher64> {
    table: ArchivedHashTable<Entry<K, V>>,
    _phantom: PhantomData<H>,
}

impl<K, V, H> ArchivedHashMap<K, V, H> {
    /// Returns whether the hash map is empty.
    pub const fn is_empty(&self) -> bool {
        self.table.is_empty()
    }

    /// Returns the number of elements in the hash map.
    pub const fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns the total capacity of the hash map.
    pub fn capacity(&self) -> usize {
        self.table.capacity()
    }

    /// Returns an iterator over the key-value entries in the hash map.
    pub fn iter(&self) -> Iter<'_, K, V, H> {
        Iter {
            raw: self.table.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the sealed key-value entries in the hash map.
    pub fn iter_seal(this: Seal<'_, Self>) -> IterMut<'_, K, V, H> {
        munge!(let Self { table, .. } = this);
        IterMut {
            raw: ArchivedHashTable::raw_iter_seal(table),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the keys in the hash map.
    pub fn keys(&self) -> Keys<'_, K, V, H> {
        Keys {
            raw: self.table.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the values in the hash map.
    pub fn values(&self) -> Values<'_, K, V, H> {
        Values {
            raw: self.table.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the mutable values in the hash map.
    pub fn values_seal(this: Seal<'_, Self>) -> ValuesMut<'_, K, V, H> {
        munge!(let Self { table, .. } = this);
        ValuesMut {
            raw: ArchivedHashTable::raw_iter_seal(table),
            _phantom: PhantomData,
        }
    }
}

impl<K, V, H: Hasher + Default> ArchivedHashMap<K, V, H> {
    /// Returns the key-value pair corresponding to the supplied key using the
    /// given comparison function.
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
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_key_value_with(key, |q, k| q == k.borrow())
    }

    /// Returns a reference to the value corresponding to the supplied key using
    /// the given comparison function.
    pub fn get_with<Q, C>(&self, key: &Q, cmp: C) -> Option<&V>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        Some(self.get_key_value_with(key, cmp)?.1)
    }

    /// Returns a reference to the value corresponding to the supplied key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_key_value(key)?.1)
    }

    /// Returns the mutable key-value pair corresponding to the supplied key
    /// using the given comparison function.
    pub fn get_key_value_seal_with<'a, Q, C>(
        this: Seal<'a, Self>,
        key: &Q,
        cmp: C,
    ) -> Option<(&'a K, Seal<'a, V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        munge!(let Self { table, .. } = this);
        let entry = ArchivedHashTable::get_seal_with(
            table,
            hash_value::<Q, H>(key),
            |e| cmp(key, &e.key),
        )?;
        munge!(let Entry { key, value } = entry);
        Some((key.unseal_ref(), value))
    }

    /// Returns the mutable key-value pair corresponding to the supplied key.
    pub fn get_key_value_seal<'a, Q>(
        this: Seal<'a, Self>,
        key: &Q,
    ) -> Option<(&'a K, Seal<'a, V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Self::get_key_value_seal_with(this, key, |q, k| q == k.borrow())
    }

    /// Returns a mutable reference to the value corresponding to the supplied
    /// key using the given comparison function.
    pub fn get_seal_with<'a, Q, C>(
        this: Seal<'a, Self>,
        key: &Q,
        cmp: C,
    ) -> Option<Seal<'a, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        Some(Self::get_key_value_seal_with(this, key, cmp)?.1)
    }

    /// Returns a mutable reference to the value corresponding to the supplied
    /// key.
    pub fn get_seal<'a, Q>(this: Seal<'a, Self>, key: &Q) -> Option<Seal<'a, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(Self::get_key_value_seal(this, key)?.1)
    }

    /// Returns whether the hash map contains the given key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Serializes an iterator of key-value pairs as a hash map.
    pub fn serialize_from_iter<I, BKU, BVU, KU, VU, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<HashMapResolver, S::Error>
    where
        I: Clone + ExactSizeIterator<Item = (BKU, BVU)>,
        BKU: Borrow<KU>,
        BVU: Borrow<VU>,
        KU: Serialize<S, Archived = K> + Hash + Eq,
        VU: Serialize<S, Archived = V>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        ArchivedHashTable::<Entry<K, V>>::serialize_from_iter(
            iter.clone()
                .map(|(key, value)| EntryAdapter::new(key, value)),
            iter.map(|(key, _)| hash_value::<KU, H>(key.borrow())),
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
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|(key, value)| {
                other.get(key).is_some_and(|v| *value == *v)
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

    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { entry.as_ref() };
            (&entry.key, &entry.value)
        })
    }
}

impl<K, V, H> ExactSizeIterator for Iter<'_, K, V, H> {
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
    type Item = (&'a K, Seal<'a, V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|mut entry| {
            let entry = unsafe { entry.as_mut() };
            (&entry.key, Seal::new(&mut entry.value))
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
    type Item = Seal<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|mut entry| {
            let entry = unsafe { entry.as_mut() };
            Seal::new(&mut entry.value)
        })
    }
}

impl<K, V, H> ExactSizeIterator for ValuesMut<'_, K, V, H> {
    fn len(&self) -> usize {
        self.raw.len()
    }
}

impl<K, V, H> FusedIterator for ValuesMut<'_, K, V, H> {}
