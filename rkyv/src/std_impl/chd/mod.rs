#[cfg(feature = "validation")]
pub mod validation;

use core::{
    borrow::Borrow,
    cmp::Reverse,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    mem::size_of,
    ops::Index,
    pin::Pin,
    slice,
};
use std::{collections::{HashMap, HashSet}};
use crate::{Archive, Archived, offset_of, RelPtr, Resolve, Unarchive, Write};

struct Entry<K, V> {
    key: K,
    value: V,
}

/// An archived `HashMap`. This is a minimal perfect hash map built using
/// [compress-hash-displace](http://cmph.sourceforge.net/papers/esa09.pdf).
#[cfg_attr(feature = "strict", repr(C))]
#[derive(Debug)]
pub struct ArchivedHashMap<K, V> {
    len: u32,
    displace: RelPtr,
    entries: RelPtr,
    phantom: PhantomData<(K, V)>,
}

impl<K: Hash + Eq, V> ArchivedHashMap<K, V> {
    /// Gets the number of items in the hash map.
    #[inline]
    pub fn len(&self) -> usize {
        self.len as usize
    }

    fn make_hasher() -> seahash::SeaHasher {
        seahash::SeaHasher::with_seeds(
            0x08576fb6170b5f5f,
            0x587775eeb84a7e46,
            0xac701115428ee569,
            0x910feb91b92bb1cd,
        )
    }

    pub fn hasher(&self) -> seahash::SeaHasher {
        Self::make_hasher()
    }

    unsafe fn displace(&self, index: usize) -> u32 {
        *self.displace.as_ptr::<u32>().add(index)
    }

    unsafe fn entry(&self, index: usize) -> &Entry<K, V> {
        &*self.entries.as_ptr::<Entry<K, V>>().add(index)
    }

    unsafe fn entry_mut(&mut self, index: usize) -> &mut Entry<K, V> {
        &mut *self.entries.as_mut_ptr::<Entry<K, V>>().add(index)
    }

    #[inline]
    fn index<Q: ?Sized>(&self, k: &Q) -> Option<usize>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        let mut hasher = self.hasher();
        k.hash(&mut hasher);
        let displace_index = hasher.finish() % self.len as u64;
        let displace = unsafe { self.displace(displace_index as usize) };

        let index = if displace == u32::MAX {
            return None;
        } else if displace & 0x80_00_00_00 == 0 {
            displace as u64
        } else {
            let mut hasher = self.hasher();
            displace.hash(&mut hasher);
            k.hash(&mut hasher);
            hasher.finish() % self.len as u64
        };

        let entry = unsafe { self.entry(index as usize) };
        if entry.key.borrow() == k {
            Some(index as usize)
        } else {
            None
        }
    }

    /// Find the key-value entry for a key.
    #[inline]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.index(k).map(move |index| {
            let entry = unsafe { self.entry(index) };
            (&entry.key, &entry.value)
        })
    }

    /// Find the mutable key-value entry for a key.
    #[inline]
    pub fn get_key_value_pin<Q: ?Sized>(self: Pin<&mut Self>, k: &Q) -> Option<(&K, Pin<&mut V>)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        unsafe {
            let hash_map = self.get_unchecked_mut();
            hash_map.index(k).map(move |index| {
                let entry = hash_map.entry_mut(index);
                (&entry.key, Pin::new_unchecked(&mut entry.value))
            })
        }
    }

    /// Returns whether a key is present in the hash map.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.index(k).is_some()
    }

    /// Gets the value associated with the given key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.index(k).map(|index| unsafe { &self.entry(index).value })
    }

    /// Gets the mutable value associated with the given key.
    #[inline]
    pub fn get_pin<Q: ?Sized>(self: Pin<&mut Self>, k: &Q) -> Option<Pin<&mut V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        unsafe {
            let hash_map = self.get_unchecked_mut();
            hash_map.index(k).map(move |index| {
                Pin::new_unchecked(&mut hash_map.entry_mut(index).value)
            })
        }
    }

    /// Returns whether there are no items in the hashmap.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    fn raw_iter(&self) -> RawIter<'_, K, V> {
        RawIter::new(self.entries.as_ptr(), self.len())
    }

    #[inline]
    fn raw_iter_pin(self: Pin<&mut Self>) -> RawIterPin<'_, K, V> {
        unsafe {
            let hash_map = self.get_unchecked_mut();
            RawIterPin::new(hash_map.entries.as_mut_ptr(), hash_map.len())
        }
    }

    /// Gets an iterator over the key-value entries in the hash map.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.raw_iter(),
        }
    }

    /// Gets an iterator over the mutable key-value entries in the hash map.
    #[inline]
    pub fn iter_pin(self: Pin<&mut Self>) -> IterPin<'_, K, V> {
        IterPin {
            inner: self.raw_iter_pin(),
        }
    }

    /// Gets an iterator over the keys in the hash map.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: self.raw_iter(),
        }
    }

    /// Gets an iterator over the values in the hash map.
    #[inline]
    pub fn values(&self) -> Values<K, V> {
        Values {
            inner: self.raw_iter(),
        }
    }

    /// Gets an iterator over the mutable values in the hash map.
    #[inline]
    pub fn values_pin(self: Pin<&mut Self>) -> ValuesPin<'_, K, V> {
        ValuesPin {
            inner: self.raw_iter_pin(),
        }
    }

    #[inline]
    fn archive_from_iter<
        'a,
        KU: 'a + Archive<Archived = K> + Hash + Eq,
        VU: 'a + Archive<Archived = V>,
        W: Write + ?Sized,
    >(
        iter: impl Iterator<Item = (&'a KU, &'a VU)>,
        len: usize,
        writer: &mut W
    ) -> Result<ArchivedHashMapResolver, W::Error> {
        let mut bucket_size = vec![0u32; len];
        let mut displaces = Vec::with_capacity(len);

        for (key, value) in iter {
            let mut hasher = Self::make_hasher();
            key.hash(&mut hasher);
            let displace = (hasher.finish() % len as u64) as u32;
            displaces.push((displace, (key, value)));
            bucket_size[displace as usize] += 1;
        }

        displaces.sort_by_key(|&(displace, _)| (Reverse(bucket_size[displace as usize]), displace));

        let mut entries = Vec::with_capacity(len);
        entries.resize_with(len, || None);
        let mut displacements = vec![u32::MAX; len];

        let mut first_empty = 0;
        let mut assignments = Vec::with_capacity(8);

        let mut start = 0;
        while start < displaces.len() {
            let displace = displaces[start].0;
            let bucket_size = bucket_size[displace as usize] as usize;
            let end = start + bucket_size;
            let bucket = &displaces[start..end];
            start = end;

            if bucket_size > 1 {
                'find_seed: for seed in 0x80_00_00_00..=0xFF_FF_FF_FF {
                    let mut base_hasher = Self::make_hasher();
                    seed.hash(&mut base_hasher);

                    assignments.clear();

                    for &(_, (key, _)) in bucket.iter() {
                        let mut hasher = base_hasher.clone();
                        key.hash(&mut hasher);
                        let index = (hasher.finish() % len as u64) as u32;
                        if entries[index as usize].is_some() || assignments.contains(&index) {
                            continue 'find_seed;
                        } else {
                            assignments.push(index);
                        }
                    }

                    for i in 0..bucket_size {
                        entries[assignments[i] as usize] = Some(bucket[i].1);
                    }
                    displacements[displace as usize] = seed;
                    break;
                }
            } else {
                let offset = entries[first_empty..].iter().position(|value| value.is_none()).unwrap();
                first_empty += offset;
                entries[first_empty] = Some(bucket[0].1);
                displacements[displace as usize] = first_empty as u32;
                first_empty += 1;
            }
        }

        // Archive entries
        let mut resolvers = entries.iter().map(|e| {
            let (key, value) = e.unwrap();
            Ok((key.archive(writer)?, value.archive(writer)?))
        }).collect::<Result<Vec<_>, _>>()?;

        // Write blocks
        let displace_pos = writer.align_for::<u32>()?;
        let displacements_slice = unsafe { slice::from_raw_parts(displacements.as_ptr().cast::<u8>(), displacements.len() * size_of::<u32>()) };
        writer.write(displacements_slice)?;

        let entries_pos = writer.align_for::<Entry<K, V>>()?;
        for ((key, value), (key_resolver, value_resolver)) in entries.iter().map(|r| r.unwrap()).zip(resolvers.drain(..)) {
            let entry_pos = writer.pos();
            let entry = Entry {
                key: key_resolver.resolve(entry_pos + offset_of!(Entry<K, V>, key), key),
                value: value_resolver.resolve(entry_pos + offset_of!(Entry<K, V>, value), value),
            };
            let entry_slice = unsafe { slice::from_raw_parts((&entry as *const Entry<K, V>).cast::<u8>(), size_of::<Entry<K, V>>()) };
            writer.write(entry_slice)?;
        }

        Ok(ArchivedHashMapResolver {
            displace_pos,
            entries_pos,
        })
    }
}

struct RawIter<'a, K: Hash + Eq, V> {
    current: *const Entry<K, V>,
    remaining: usize,
    _phantom: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Hash + Eq, V> RawIter<'a, K, V> {
    fn new(pairs: *const Entry<K, V>, len: usize) -> Self {
        Self {
            current: pairs,
            remaining: len,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: Hash + Eq, V> Iterator for RawIter<'a, K, V> {
    type Item = *const Entry<K, V>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.remaining == 0 {
                None
            } else {
                let result = self.current;
                self.current = self.current.add(1);
                self.remaining -= 1;
                Some(result)
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K: Hash + Eq, V> ExactSizeIterator for RawIter<'a, K, V> {}
impl<'a, K: Hash + Eq, V> FusedIterator for RawIter<'a, K, V> {}

struct RawIterPin<'a, K: Hash + Eq, V> {
    current: *mut Entry<K, V>,
    remaining: usize,
    _phantom: PhantomData<(&'a K, Pin<&'a mut V>)>,
}

impl<'a, K: Hash + Eq, V> RawIterPin<'a, K, V> {
    fn new(pairs: *mut Entry<K, V>, len: usize) -> Self {
        Self {
            current: pairs,
            remaining: len,
            _phantom: PhantomData,
        }
    }
}

impl<'a, K: Hash + Eq, V> Iterator for RawIterPin<'a, K, V> {
    type Item = *mut Entry<K, V>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if self.remaining == 0 {
                None
            } else {
                let result = self.current;
                self.current = self.current.add(1);
                self.remaining -= 1;
                Some(result)
            }
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for RawIterPin<'_, K, V> {}
impl<K: Hash + Eq, V> FusedIterator for RawIterPin<'_, K, V> {}

/// An iterator over the key-value pairs of a hash map.
#[repr(transparent)]
pub struct Iter<'a, K: Hash + Eq, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let pair = &*x;
            (&pair.key, &pair.value)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for Iter<'_, K, V> {}
impl<K: Hash + Eq, V> FusedIterator for Iter<'_, K, V> {}

/// An iterator over the mutable key-value pairs of a hash map.
#[repr(transparent)]
pub struct IterPin<'a, K: Hash + Eq, V> {
    inner: RawIterPin<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for IterPin<'a, K, V> {
    type Item = (&'a K, Pin<&'a mut V>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let pair = &mut *x;
            (&pair.key, Pin::new_unchecked(&mut pair.value))
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for IterPin<'_, K, V> {}
impl<K: Hash + Eq, V> FusedIterator for IterPin<'_, K, V> {}

/// An iterator over the keys of a hash map.
#[repr(transparent)]
pub struct Keys<'a, K: Hash + Eq, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let pair = &*x;
            &pair.key
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for Keys<'_, K, V> {}
impl<K: Hash + Eq, V> FusedIterator for Keys<'_, K, V> {}

/// An iterator over the values of a hash map.
#[repr(transparent)]
pub struct Values<'a, K: Hash + Eq, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let pair = &*x;
            &pair.value
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for Values<'_, K, V> {}
impl<K: Hash + Eq, V> FusedIterator for Values<'_, K, V> {}

/// An iterator over the mutable values of a hash map.
#[repr(transparent)]
pub struct ValuesPin<'a, K: Hash + Eq, V> {
    inner: RawIterPin<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for ValuesPin<'a, K, V> {
    type Item = Pin<&'a mut V>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let pair = &mut *x;
            Pin::new_unchecked(&mut pair.value)
        })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for ValuesPin<'_, K, V> {}
impl<K: Hash + Eq, V> FusedIterator for ValuesPin<'_, K, V> {}

/// The resolver for archived hash maps.
pub struct ArchivedHashMapResolver {
    displace_pos: usize,
    entries_pos: usize,
}

impl ArchivedHashMapResolver {
    fn resolve_from_len<K, V>(self, pos: usize, len: usize) -> ArchivedHashMap<K, V> {
        unsafe {
            ArchivedHashMap {
                len: len as u32,
                displace: RelPtr::new(pos + offset_of!(ArchivedHashMap<K, V>, displace), self.displace_pos),
                entries: RelPtr::new(pos + offset_of!(ArchivedHashMap<K, V>, entries), self.entries_pos),
                phantom: PhantomData,
            }
        }
    }
}

impl<K: Archive + Hash + Eq, V: Archive> Resolve<HashMap<K, V>> for ArchivedHashMapResolver {
    type Archived = ArchivedHashMap<K::Archived, V::Archived>;

    fn resolve(self, pos: usize, value: &HashMap<K, V>) -> Self::Archived {
        self.resolve_from_len(pos, value.len())
    }
}

impl<K: Archive + Hash + Eq, V: Archive> Archive for HashMap<K, V>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashMap<K::Archived, V::Archived>;
    type Resolver = ArchivedHashMapResolver;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(ArchivedHashMap::archive_from_iter(self.iter(), self.len(), writer)?)
    }
}

impl<K: Archive + Hash + Eq, V: Archive> Unarchive<HashMap<K, V>> for Archived<HashMap<K, V>>
where
    K::Archived: Unarchive<K> + Hash + Eq,
    V::Archived: Unarchive<V>,
{
    fn unarchive(&self) -> HashMap<K, V> {
        let mut result = HashMap::new();
        for (k, v) in self.iter() {
            result.insert(k.unarchive(), v.unarchive());
        }
        result
    }
}

impl<K: Hash + Eq, V: PartialEq> PartialEq for ArchivedHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
        }
    }
}

impl<K: Hash + Eq, V: Eq> Eq for ArchivedHashMap<K, V> {}

impl<K: Hash + Eq + Borrow<AK>, V, AK: Hash + Eq, AV: PartialEq<V>> PartialEq<HashMap<K, V>> for ArchivedHashMap<AK, AV> {
    fn eq(&self, other: &HashMap<K, V>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
        }
    }
}

impl<K: Hash + Eq + Borrow<AK>, V, AK: Hash + Eq, AV: PartialEq<V>> PartialEq<ArchivedHashMap<AK, AV>> for HashMap<K, V> {
    fn eq(&self, other: &ArchivedHashMap<AK, AV>) -> bool {
        other.eq(self)
    }
}

impl<K: Eq + Hash + Borrow<Q>, Q: Eq + Hash + ?Sized, V> Index<&'_ Q> for ArchivedHashMap<K, V> {
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).unwrap()
    }
}

/// An archived `HashSet`. This is a wrapper around a hash map with the same key
/// and a value of `()`.
#[derive(Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ArchivedHashSet<K: Hash + Eq>(ArchivedHashMap<K, ()>);

impl<K: Hash + Eq> ArchivedHashSet<K> {
    #[inline]
    pub fn len(&self) -> usize {
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

    /// Gets the hasher for the underlying hash map.
    #[inline]
    pub fn hasher(&self) -> seahash::SeaHasher {
        self.0.hasher()
    }

    /// Returns whether there are no items in the hash set.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets an iterator over the keys of the underlying hash map.
    #[inline]
    pub fn iter(&self) -> Keys<K, ()> {
        self.0.keys()
    }
}

/// The resolver for archived hash sets.
pub struct ArchivedHashSetResolver(ArchivedHashMapResolver);

impl<K: Archive + Hash + Eq> Resolve<HashSet<K>> for ArchivedHashSetResolver
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashSet<K::Archived>;

    fn resolve(self, pos: usize, value: &HashSet<K>) -> Self::Archived {
        ArchivedHashSet(self.0.resolve_from_len(pos, value.len()))
    }
}

impl<K: Archive + Hash + Eq> Archive for HashSet<K>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashSet<K::Archived>;
    type Resolver = ArchivedHashSetResolver;

    fn archive<W: Write + ?Sized>(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(ArchivedHashSetResolver(ArchivedHashMap::archive_from_iter(self.iter().map(|x| (x, &())), self.len(), writer)?))
    }
}
