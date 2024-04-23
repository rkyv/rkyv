//! An archived index map implementation based on Google's high-performance
//! SwissTable hash map.

use core::{
    borrow::Borrow,
    fmt,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    pin::Pin,
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
    primitive::ArchivedUsize,
    ser::{Allocator, Writer, WriterExt as _},
    Place, Portable, RelPtr, Serialize,
};

/// An archived `IndexMap`.
#[derive(Portable)]
#[archive(crate)]
#[repr(C)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    check_bytes(verify)
)]
pub struct ArchivedIndexMap<K, V, H = FxHasher64> {
    table: ArchivedHashTable<ArchivedUsize>,
    entries: RelPtr<Entry<K, V>>,
    _phantom: PhantomData<H>,
}

impl<K, V, H> ArchivedIndexMap<K, V, H> {
    fn entries(&self) -> &[Entry<K, V>] {
        unsafe { from_raw_parts(self.entries.as_ptr(), self.len()) }
    }

    fn entries_mut(self: Pin<&mut Self>) -> Pin<&mut [Entry<K, V>]> {
        let len = self.len();
        unsafe {
            Pin::map_unchecked_mut(self, |s| {
                from_raw_parts_mut(s.entries.as_ptr(), len)
            })
        }
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    unsafe fn raw_iter(&self) -> RawIter<K, V> {
        unsafe { RawIter::new(self.entries.as_ptr().cast(), self.len()) }
    }

    /// Returns an iterator over the key-value pairs of the map in order
    #[inline]
    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            inner: unsafe { self.raw_iter() },
        }
    }

    /// Returns an iterator over the keys of the map in order
    #[inline]
    pub fn keys(&self) -> Keys<K, V> {
        Keys {
            inner: unsafe { self.raw_iter() },
        }
    }

    /// Gets the number of items in the index map.
    #[inline]
    pub const fn len(&self) -> usize {
        self.table.len()
    }

    /// Returns an iterator over the values of the map in order.
    #[inline]
    pub fn values(&self) -> Values<K, V> {
        Values {
            inner: unsafe { self.raw_iter() },
        }
    }
}

impl<K, V, H: Hasher + Default> ArchivedIndexMap<K, V, H> {
    /// Gets the index, key, and value corresponding to the supplied key using
    /// the given comparison function.
    #[inline]
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
    #[inline]
    pub fn get_full<Q>(&self, key: &Q) -> Option<(usize, &K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_full_with(key, |q, k| q == k.borrow())
    }

    /// Returns the key-value pair corresponding to the supplied key using the
    /// given comparison function.
    #[inline]
    pub fn get_key_value_with<Q, C>(&self, key: &Q, cmp: C) -> Option<(&K, &V)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let (_, k, v) = self.get_full_with(key, cmp)?;
        Some((k, v))
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
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
    #[inline]
    pub fn get_with<Q, C>(&self, key: &Q, cmp: C) -> Option<&V>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        Some(self.get_full_with(key, cmp)?.2)
    }

    /// Returns a reference to the value corresponding to the supplied key.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_full(key)?.2)
    }

    /// Gets the mutable index, key, and value corresponding to the supplied key
    /// using the given comparison function.
    #[inline]
    pub fn get_full_with_mut<Q, C>(
        self: Pin<&mut Self>,
        key: &Q,
        cmp: C,
    ) -> Option<(usize, &K, Pin<&mut V>)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&Q, &K) -> bool,
    {
        let index = self.get_index_of_with(key, cmp)?;
        let entries = unsafe { Pin::into_inner_unchecked(self.entries_mut()) };
        let entry = &mut entries[index];
        let value = unsafe { Pin::new_unchecked(&mut entry.value) };
        Some((index, &entry.key, value))
    }

    /// Gets the mutable index, key, and value corresponding to the supplied
    /// key.
    #[inline]
    pub fn get_full_mut<Q>(
        self: Pin<&mut Self>,
        key: &Q,
    ) -> Option<(usize, &K, Pin<&mut V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get_full_with_mut(key, |q, k| q == k.borrow())
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
        let (_, k, v) = self.get_full_with_mut(key, cmp)?;
        Some((k, v))
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
        let (_, k, v) = self.get_full_mut(key)?;
        Some((k, v))
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
        Some(self.get_full_with_mut(key, cmp)?.2)
    }

    /// Returns a mutable reference to the value corresponding to the supplied
    /// key.
    #[inline]
    pub fn get_mut<Q>(self: Pin<&mut Self>, key: &Q) -> Option<Pin<&mut V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_full_mut(key)?.2)
    }

    /// Returns whether a key is present in the hash map.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Gets a key-value pair by index.
    #[inline]
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
    #[inline]
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
    #[inline]
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
        RelPtr::emplace(resolver.entries_pos, entries);
    }

    /// Serializes an iterator of key-value pairs as an index map.
    pub fn serialize_from_iter<'a, I, UK, UV, S>(
        iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<IndexMapResolver, S::Error>
    where
        I: Clone + ExactSizeIterator<Item = (&'a UK, &'a UV)>,
        UK: 'a + Serialize<S, Archived = K> + Hash + Eq,
        UV: 'a + Serialize<S, Archived = V>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        use crate::util::SerVec;

        // Serialize hash table
        let table_resolver =
            ArchivedHashTable::<ArchivedUsize>::serialize_from_iter(
                0..iter.len(),
                iter.clone().map(|(key, _)| hash_value::<UK, H>(key)),
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
                        key: key.serialize(serializer)?,
                        value: value.serialize(serializer)?,
                    });
                }

                let entries_pos = serializer.align_for::<Entry<K, V>>()?;
                for ((key, value), resolver) in
                    iter.clone().zip(resolvers.drain(..))
                {
                    unsafe {
                        serializer.resolve_aligned(
                            &EntryAdapter { key, value },
                            resolver,
                        )?;
                    }
                }

                Ok(IndexMapResolver {
                    table_resolver,
                    entries_pos,
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

impl<'a, K, V> RawIter<'a, K, V> {
    #[inline]
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

    #[inline]
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

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> ExactSizeIterator for RawIter<'a, K, V> {}
impl<'a, K, V> FusedIterator for RawIter<'a, K, V> {}

/// An iterator over the key-value pairs of an index map.
#[repr(transparent)]
pub struct Iter<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
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

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {}
impl<K, V> FusedIterator for Values<'_, K, V> {}

/// The resolver for an `IndexMap`.
pub struct IndexMapResolver {
    table_resolver: HashTableResolver,
    entries_pos: usize,
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
            let ptr = unsafe {
                context.bounds_check_subtree_base_offset::<[Entry<K, V>]>(
                    self.entries.base(),
                    self.entries.offset(),
                    self.table.len(),
                )?
            };

            let range = unsafe { context.push_prefix_subtree(ptr)? };
            unsafe {
                <[Entry<K, V>]>::check_bytes(ptr, context)?;
            }
            unsafe {
                context.pop_subtree_range(range)?;
            }

            Ok(())
        }
    }
}
