//! An archived index map implementation based on Google's high-performance
//! SwissTable hash map.

use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    slice::{from_raw_parts, from_raw_parts_mut},
};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    collections::{
        swiss_table::{ArchivedHashTable, HashTableResolver},
        util::{Entry, EntryAdapter, EntryResolver},
    },
    hash::{hash_value, FxHasher64},
    primitive::{ArchivedUsize, FixedUsize},
    seal::Seal,
    ser::{Allocator, Writer, WriterExt as _},
    Place, Portable, RelPtr, Serialize,
};

/// An archived `IndexMap`.
#[derive(Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedIndexMap<K, V, H = FxHasher64> {
    table: ArchivedHashTable<ArchivedUsize>,
    entries: RelPtr<Entry<K, V>>,
    _phantom: PhantomData<H>,
}

impl<K, V, H> ArchivedIndexMap<K, V, H> {
    fn entries(&self) -> &[Entry<K, V>] {
        unsafe { from_raw_parts(self.entries.as_ptr(), self.len()) }
    }

    fn entries_seal(this: Seal<'_, Self>) -> Seal<'_, [Entry<K, V>]> {
        let len = this.len();
        munge!(let Self { entries, .. } = this);
        let slice =
            unsafe { from_raw_parts_mut(RelPtr::as_mut_ptr(entries), len) };
        Seal::new(slice)
    }

    /// Returns `true` if the map contains no elements.
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    unsafe fn raw_iter(&self) -> RawIter<'_, K, V> {
        unsafe { RawIter::new(self.entries.as_ptr().cast(), self.len()) }
    }

    /// Returns an iterator over the key-value pairs of the map in order
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: unsafe { self.raw_iter() },
        }
    }

    /// Returns an iterator over the keys of the map in order
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: unsafe { self.raw_iter() },
        }
    }

    /// Gets the number of items in the index map.
    pub const fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns an iterator over the values of the map in order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            inner: unsafe { self.raw_iter() },
        }
    }
}

impl<K, V, H: Hasher + Default> ArchivedIndexMap<K, V, H> {
    /// Gets the index, key, and value corresponding to the supplied key using
    /// the given comparison function.
    pub fn get_full_with<Q, C>(
        &self,
        key: &Q,
        cmp: C,
    ) -> Option<(usize, &K, &V)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let index = self.get_index_of_with(key, cmp)?;
        let entries = self.entries();
        let entry = &entries[index];
        Some((index, &entry.key, &entry.value))
    }

    /// Gets the index, key, and value corresponding to the supplied key.
    pub fn get_full<Q>(&self, key: &Q) -> Option<(usize, &K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_full_with(key, |q, k| q == k.borrow())
    }

    /// Returns the key-value pair corresponding to the supplied key using the
    /// given comparison function.
    pub fn get_key_value_with<Q, C>(&self, key: &Q, cmp: C) -> Option<(&K, &V)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let (_, k, v) = self.get_full_with(key, cmp)?;
        Some((k, v))
    }

    /// Returns the key-value pair corresponding to the supplied key.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let (_, k, v) = self.get_full(key)?;
        Some((k, v))
    }

    /// Returns a reference to the value corresponding to the supplied key using
    /// the given comparison function.
    pub fn get_with<Q, C>(&self, key: &Q, cmp: C) -> Option<&V>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        Some(self.get_full_with(key, cmp)?.2)
    }

    /// Returns a reference to the value corresponding to the supplied key.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_full(key)?.2)
    }

    /// Gets the mutable index, key, and value corresponding to the supplied key
    /// using the given comparison function.
    pub fn get_full_seal_with<'a, Q, C>(
        this: Seal<'a, Self>,
        key: &Q,
        cmp: C,
    ) -> Option<(usize, &'a K, Seal<'a, V>)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let index = this.get_index_of_with(key, cmp)?;
        let entry = Seal::index(Self::entries_seal(this), index);
        munge!(let Entry { key, value } = entry);
        Some((index, key.unseal_ref(), value))
    }

    /// Gets the mutable index, key, and value corresponding to the supplied
    /// key.
    pub fn get_full_seal<'a, Q>(
        this: Seal<'a, Self>,
        key: &Q,
    ) -> Option<(usize, &'a K, Seal<'a, V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Self::get_full_seal_with(this, key, |q, k| q == k.borrow())
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
        let (_, k, v) = Self::get_full_seal_with(this, key, cmp)?;
        Some((k, v))
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
        let (_, k, v) = Self::get_full_seal(this, key)?;
        Some((k, v))
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
        Some(Self::get_full_seal_with(this, key, cmp)?.2)
    }

    /// Returns a mutable reference to the value corresponding to the supplied
    /// key.
    pub fn get_seal<'a, Q>(this: Seal<'a, Self>, key: &Q) -> Option<Seal<'a, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(Self::get_full_seal(this, key)?.2)
    }

    /// Returns whether a key is present in the hash map.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Gets a key-value pair by index.
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        if index < self.len() {
            let entry = &self.entries()[index];
            Some((&entry.key, &entry.value))
        } else {
            None
        }
    }

    /// Gets the index of a key if it exists in the map using the given
    /// comparison function.
    pub fn get_index_of_with<Q, C>(&self, key: &Q, cmp: C) -> Option<usize>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let entries = self.entries();
        let index = self.table.get_with(hash_value::<Q, H>(key), |i| {
            cmp(key, &entries[i.to_native() as usize].key)
        })?;
        Some(index.to_native() as usize)
    }

    /// Gets the index of a key if it exists in the map.
    pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_index_of_with(key, |q, k| q == k.borrow())
    }

    /// Resolves an archived index map from a given length and parameters.
    pub fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        resolver: IndexMapResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedIndexMap { table, entries, _phantom: _ } = out);
        ArchivedHashTable::resolve_from_len(
            len,
            load_factor,
            resolver.table_resolver,
            table,
        );
        RelPtr::emplace(resolver.entries_pos as usize, entries);
    }

    /// Serializes an iterator of key-value pairs as an index map.
    pub fn serialize_from_iter<I, BKU, BVU, KU, VU, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<IndexMapResolver, S::Error>
    where
        I: Clone + ExactSizeIterator<Item = (BKU, BVU)>,
        BKU: Borrow<KU>,
        BVU: Borrow<VU>,
        KU: Serialize<S, Archived = K> + Hash + Eq,
        VU: Serialize<S, Archived = V>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        use crate::util::SerVec;

        // Serialize hash table
        let table_resolver =
            ArchivedHashTable::<ArchivedUsize>::serialize_from_iter(
                0..iter.len(),
                iter.clone()
                    .map(|(key, _)| hash_value::<KU, H>(key.borrow())),
                load_factor,
                serializer,
            )?;

        // Serialize entries
        SerVec::with_capacity(
            serializer,
            iter.len(),
            |resolvers, serializer| {
                for (key, value) in iter.clone() {
                    resolvers.push(EntryResolver {
                        key: key.borrow().serialize(serializer)?,
                        value: value.borrow().serialize(serializer)?,
                    });
                }

                let entries_pos = serializer.align_for::<Entry<K, V>>()?;
                for ((key, value), resolver) in
                    iter.clone().zip(resolvers.drain())
                {
                    unsafe {
                        serializer.resolve_aligned(
                            &EntryAdapter::new(key, value),
                            resolver,
                        )?;
                    }
                }

                Ok(IndexMapResolver {
                    table_resolver,
                    entries_pos: entries_pos as FixedUsize,
                })
            },
        )?
    }
}

impl<K, V, H> fmt::Debug for ArchivedIndexMap<K, V, H>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, V, H> PartialEq for ArchivedIndexMap<K, V, H>
where
    K: PartialEq,
    V: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}

impl<K: Eq, V: Eq, H> Eq for ArchivedIndexMap<K, V, H> {}

struct RawIter<'a, K, V> {
    current: *const Entry<K, V>,
    remaining: usize,
    _phantom: PhantomData<(&'a K, &'a V)>,
}

impl<K, V> RawIter<'_, K, V> {
    fn new(pairs: *const Entry<K, V>, len: usize) -> Self {
        Self {
            current: pairs,
            remaining: len,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K, V> Iterator for RawIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.remaining == 0 {
                None
            } else {
                let result = self.current;
                self.current = self.current.add(1);
                self.remaining -= 1;
                let entry = &*result;
                Some((&entry.key, &entry.value))
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K, V> ExactSizeIterator for RawIter<'_, K, V> {}
impl<K, V> FusedIterator for RawIter<'_, K, V> {}

/// An iterator over the key-value pairs of an index map.
#[repr(transparent)]
pub struct Iter<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {}
impl<K, V> FusedIterator for Iter<'_, K, V> {}

/// An iterator over the keys of an index map.
#[repr(transparent)]
pub struct Keys<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {}
impl<K, V> FusedIterator for Keys<'_, K, V> {}

/// An iterator over the values of an index map.
#[repr(transparent)]
pub struct Values<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {}
impl<K, V> FusedIterator for Values<'_, K, V> {}

/// The resolver for an `IndexMap`.
pub struct IndexMapResolver {
    table_resolver: HashTableResolver,
    entries_pos: FixedUsize,
}

#[cfg(feature = "bytecheck")]
mod verify {
    use bytecheck::{CheckBytes, Verify};
    use rancor::{Fallible, Source};

    use super::ArchivedIndexMap;
    use crate::{
        collections::util::Entry,
        validation::{ArchiveContext, ArchiveContextExt},
    };

    unsafe impl<C, K, V, H> Verify<C> for ArchivedIndexMap<K, V, H>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        fn verify(
            &self,
            context: &mut C,
        ) -> Result<(), <C as Fallible>::Error> {
            let ptr = core::ptr::slice_from_raw_parts(
                self.entries.as_ptr_wrapping(),
                self.table.len(),
            );

            context.in_subtree(ptr, |context| {
                // SAFETY: `in_subtree` has checked that `ptr` is aligned and
                // points to enough bytes to represent its slice.
                unsafe { <[Entry<K, V>]>::check_bytes(ptr, context) }
            })
        }
    }
}
