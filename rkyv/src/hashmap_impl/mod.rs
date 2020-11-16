//! [`Archive`] implementation for [`HashMap`].

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri),
        not(feature = "more_portable")
    ))] {
        mod sse2;
        use sse2 as imp;
    } else {
        mod generic;
        use generic as imp;
    }
}
mod bitmask;

use self::bitmask::BitMask;
use self::imp::Group;
use crate::{Archive, RelPtr, Resolve, Write, WriteExt};
use core::{
    borrow::Borrow,
    cmp::Eq,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    mem,
    ops::Index,
    pin::Pin,
    ptr,
};
use memoffset::offset_of;
use std::collections::{HashMap, HashSet};

#[cfg(feature = "nightly")]
use core::intrinsics::{likely, unlikely};
#[cfg(not(feature = "nightly"))]
#[inline]
fn likely(b: bool) -> bool {
    b
}
#[cfg(not(feature = "nightly"))]
#[inline]
fn unlikely(b: bool) -> bool {
    b
}

const EMPTY: u8 = 0b1111_1111;

#[inline]
fn is_full(ctrl: u8) -> bool {
    ctrl & 0x80 == 0
}

#[inline]
fn h1(hash: u64) -> usize {
    hash as usize
}

#[inline]
fn h2(hash: u64) -> u8 {
    let top7 = hash >> (mem::size_of::<u32>() * 8 - 7);
    (top7 & 0x7f) as u8
}

struct ProbeSeq {
    bucket_mask: usize,
    pos: usize,
    stride: usize,
}

impl Iterator for ProbeSeq {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        debug_assert!(
            self.stride <= self.bucket_mask,
            "Went past end of probe sequence"
        );

        let result = self.pos;
        self.stride += Group::WIDTH;
        self.pos += self.stride;
        self.pos &= self.bucket_mask;
        Some(result)
    }
}

#[cfg_attr(feature = "inline_more", inline)]
fn capacity_to_buckets(cap: usize) -> Option<usize> {
    let adjusted_cap = if cap < 8 {
        cap + 1
    } else {
        cap.checked_mul(8)? / 7
    };

    Some(adjusted_cap.next_power_of_two())
}

#[derive(Debug)]
struct ArchivedBucket<K, V> {
    key: K,
    value: V,
}

#[doc(hidden)]
pub struct ArchivedHashMapResolver {
    ctrl_pos: usize,
    data_pos: usize,
}

impl ArchivedHashMapResolver {
    fn resolve_from_len<K, V>(self, pos: usize, len: usize) -> ArchivedHashMap<K, V> {
        let buckets = capacity_to_buckets(len).unwrap();

        ArchivedHashMap {
            bucket_mask: buckets as u32 - 1,
            ctrl: unsafe {
                RelPtr::new(pos + offset_of!(ArchivedHashMap<K, V>, ctrl), self.ctrl_pos)
            },
            data: unsafe {
                RelPtr::new(pos + offset_of!(ArchivedHashMap<K, V>, data), self.data_pos)
            },
            items: len as u32,
            marker: PhantomData,
        }
    }
}

/// An archived `HashMap`. This is a direct port of the standard library
/// `hashbrown` hash map for rkyv.
#[derive(Debug)]
pub struct ArchivedHashMap<K, V> {
    bucket_mask: u32,
    ctrl: RelPtr,
    data: RelPtr,
    items: u32,
    marker: PhantomData<(K, V)>,
}

#[inline]
fn hasher() -> seahash::SeaHasher {
    seahash::SeaHasher::with_seeds(
        0x08576fb6170b5f5f,
        0x587775eeb84a7e46,
        0xac701115428ee569,
        0x910feb91b92bb1cd,
    )
}

#[cfg_attr(feature = "inline_more", inline)]
fn make_hash<T: Hash + ?Sized>(value: &T) -> u64 {
    let mut hasher = hasher();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg_attr(feature = "inline_more", inline)]
fn probe_seq(bucket_mask: u32, hash: u64) -> ProbeSeq {
    ProbeSeq {
        bucket_mask: bucket_mask as usize,
        pos: h1(hash) & bucket_mask as usize,
        stride: 0,
    }
}

#[cfg_attr(feature = "inline_more", inline)]
fn find_insert_slot(bucket_mask: u32, hash: u64, ctrl: *const u8) -> usize {
    for pos in probe_seq(bucket_mask, hash) {
        unsafe {
            let group = Group::load(ctrl.add(pos));
            if let Some(bit) = group.match_empty().lowest_set_bit() {
                let result = (pos + bit) & bucket_mask as usize;

                if unlikely(is_full(*ctrl.add(result))) {
                    debug_assert!((bucket_mask as usize) < Group::WIDTH);
                    debug_assert_ne!(pos, 0);
                    return Group::load_aligned(ctrl)
                        .match_empty()
                        .lowest_set_bit_nonzero();
                } else {
                    return result;
                }
            }
        }
    }

    unreachable!();
}

impl<K: Hash + Eq, V> ArchivedHashMap<K, V> {
    #[cfg_attr(feature = "inline_more", inline)]
    fn buckets(&self) -> usize {
        (self.bucket_mask + 1) as usize
    }

    #[inline]
    fn find<Q: ?Sized>(&self, hash: u64, k: &Q) -> Option<&ArchivedBucket<K, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        unsafe {
            let ctrl = self.ctrl.as_ptr::<u8>();
            let data = self.data.as_ptr::<ArchivedBucket<K, V>>();
            for pos in probe_seq(self.bucket_mask, hash) {
                let group = Group::load(ctrl.add(pos));
                for bit in group.match_byte(h2(hash)) {
                    let index = (pos + bit) & self.bucket_mask as usize;
                    let bucket = &*data.add(index);
                    if likely(k.eq(bucket.key.borrow())) {
                        return Some(bucket);
                    }
                }
                if likely(group.match_empty().any_bit_set()) {
                    return None;
                }
            }
        }

        unreachable!();
    }

    #[inline]
    fn find_mut<Q: ?Sized>(&mut self, hash: u64, k: &Q) -> Option<&mut ArchivedBucket<K, V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        unsafe {
            let ctrl = self.ctrl.as_ptr::<u8>();
            let data = self.data.as_mut_ptr::<ArchivedBucket<K, V>>();
            for pos in probe_seq(self.bucket_mask, hash) {
                let group = Group::load(ctrl.add(pos));
                for bit in group.match_byte(h2(hash)) {
                    let index = (pos + bit) & self.bucket_mask as usize;
                    let bucket = &mut *data.add(index);
                    if likely(k.eq(bucket.key.borrow())) {
                        return Some(bucket);
                    }
                }
                if likely(group.match_empty().any_bit_set()) {
                    return None;
                }
            }
        }

        unreachable!();
    }

    /// Gets the number of items in the hash map.
    #[inline]
    pub fn len(&self) -> usize {
        self.items as usize
    }

    /// Finds the key-value pair for a key.
    #[inline]
    pub fn get_key_value<Q: ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.find(make_hash(k), k)
            .map(|bucket| (&bucket.key, &bucket.value))
    }

    /// Returns whether a key is present in the hash map.
    #[inline]
    pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.find(make_hash(k), k).is_some()
    }

    /// Gets the value associated with the given key.
    #[inline]
    pub fn get<Q: ?Sized>(&self, k: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        self.find(make_hash(k), k).map(|bucket| &bucket.value)
    }

    /// Get the mutable value associated with the given key.
    #[inline]
    pub fn get_pin<Q: ?Sized>(self: Pin<&mut Self>, k: &Q) -> Option<Pin<&mut V>>
    where
        K: Borrow<Q>,
        Q: Hash + Eq,
    {
        unsafe {
            let result = self
                .get_unchecked_mut()
                .find_mut(make_hash(k), k)
                .map(|bucket| &mut bucket.value);
            result.map(|value| Pin::new_unchecked(value))
        }
    }

    /// Gets the hasher for the hash map.
    #[inline]
    pub fn hasher(&self) -> seahash::SeaHasher {
        hasher()
    }

    /// Returns whether there are no items in the hash map.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn raw_iter(&self) -> RawIter<'_, K, V> {
        if self.items == 0 {
            RawIter::new(
                Group::static_empty().as_ptr(),
                ptr::NonNull::dangling().as_ptr(),
                self.buckets(),
                self.items as usize,
            )
        } else {
            let ctrl = self.ctrl.as_ptr();
            let data = self.data.as_ptr();
            RawIter::new(ctrl, data, self.buckets(), self.items as usize)
        }
    }

    fn raw_iter_pin(self: Pin<&mut Self>) -> RawIterPin<'_, K, V> {
        unsafe {
            let s = self.get_unchecked_mut();

            if s.items == 0 {
                RawIterPin::new(
                    Group::static_empty().as_ptr(),
                    ptr::NonNull::dangling().as_ptr(),
                    s.buckets(),
                    s.items as usize,
                )
            } else {
                let ctrl = s.ctrl.as_ptr();
                let data = s.data.as_mut_ptr();
                RawIterPin::new(ctrl, data, s.buckets(), s.items as usize)
            }
        }
    }

    /// Gets an iterator over the key-value pairs in the hash map.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.raw_iter(),
        }
    }

    /// Gets an iterator over the mutable key-value pairs in the hash
    /// map.
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
        items: impl Iterator<Item = (&'a KU, &'a VU)>,
        len: usize,
        writer: &mut W,
    ) -> Result<ArchivedHashMapResolver, W::Error> {
        if len == 0 {
            Ok(ArchivedHashMapResolver {
                ctrl_pos: 0,
                data_pos: 0,
            })
        } else {
            // Archive items
            let items_resolvers = items
                .map(|(k, v)| Ok(((k, v), (k.archive(writer)?, v.archive(writer)?))))
                .collect::<Result<Vec<_>, _>>()?;

            // Initialize with capacity
            let buckets = capacity_to_buckets(len).unwrap();
            debug_assert!(buckets.is_power_of_two());

            let bucket_mask = buckets as u32 - 1;
            let mut ctrl = vec![EMPTY; buckets + Group::WIDTH];
            let mut data = Vec::with_capacity(buckets);
            for _ in 0..buckets {
                data.push(mem::MaybeUninit::<ArchivedBucket<K, V>>::zeroed());
            }

            // Set up writers
            writer.align(Group::WIDTH)?;
            let ctrl_pos = writer.pos();
            let data_align = usize::max(mem::align_of::<ArchivedBucket<K, V>>(), Group::WIDTH);
            let data_pos = ctrl_pos
                .checked_add(ctrl.len())
                .unwrap()
                .checked_add(data_align - 1)
                .unwrap()
                & !(data_align - 1);

            // Insert items
            for ((key, value), (key_resolver, value_resolver)) in items_resolvers {
                let hash = make_hash(&key);

                // Find insert slot
                let ctrl_value = h2(hash);
                let index = find_insert_slot(bucket_mask, hash, ctrl.as_ptr());
                let index2 =
                    ((index.wrapping_sub(Group::WIDTH)) & bucket_mask as usize) + Group::WIDTH;
                ctrl[index] = ctrl_value;
                ctrl[index2] = ctrl_value;

                // Resolve items
                let bucket_pos = data_pos + index * mem::size_of::<ArchivedBucket<K, V>>();
                unsafe {
                    data[index].as_mut_ptr().write(ArchivedBucket {
                        key: key_resolver
                            .resolve(bucket_pos + offset_of!(ArchivedBucket<K, V>, key), key),
                        value: value_resolver
                            .resolve(bucket_pos + offset_of!(ArchivedBucket<K, V>, value), value),
                    });
                }
            }

            // Write blocks
            writer.write(ctrl.as_slice())?;
            writer.align(data_align)?;
            writer.write(unsafe {
                core::slice::from_raw_parts(
                    data.as_ptr().cast::<u8>(),
                    mem::size_of::<mem::MaybeUninit<ArchivedBucket<K, V>>>() * data.len(),
                )
            })?;

            Ok(ArchivedHashMapResolver { ctrl_pos, data_pos })
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
        ArchivedHashMap::archive_from_iter(self.iter(), self.len(), writer)
    }
}

impl<K: Hash + Eq, V: PartialEq> PartialEq for ArchivedHashMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter()
                .all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
        }
    }
}

impl<K: Hash + Eq, V: Eq> Eq for ArchivedHashMap<K, V> {}

impl<K: Hash + Eq + Borrow<AK>, V, AK: Hash + Eq, AV: PartialEq<V>> PartialEq<HashMap<K, V>>
    for ArchivedHashMap<AK, AV>
{
    fn eq(&self, other: &HashMap<K, V>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter()
                .all(|(key, value)| other.get(key).map_or(false, |v| *value == *v))
        }
    }
}

impl<K: Hash + Eq + Borrow<AK>, V, AK: Hash + Eq, AV: PartialEq<V>>
    PartialEq<ArchivedHashMap<AK, AV>> for HashMap<K, V>
{
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

struct RawIter<'a, K: Hash + Eq, V> {
    current_group: BitMask,
    data: *const ArchivedBucket<K, V>,
    next_ctrl: *const u8,
    end: *const u8,
    items: usize,
    _phantom: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Hash + Eq, V> RawIter<'a, K, V> {
    fn new(
        ctrl: *const u8,
        data: *const ArchivedBucket<K, V>,
        buckets: usize,
        items: usize,
    ) -> Self {
        debug_assert_ne!(buckets, 0);
        debug_assert_eq!(ctrl as usize % Group::WIDTH, 0);
        unsafe {
            let end = ctrl.add(buckets);

            let current_group = Group::load_aligned(ctrl).match_full();
            let next_ctrl = ctrl.add(Group::WIDTH);

            Self {
                current_group,
                data,
                next_ctrl,
                end,
                items,
                _phantom: PhantomData,
            }
        }
    }
}

impl<'a, K: Hash + Eq, V> Iterator for RawIter<'a, K, V> {
    type Item = *const ArchivedBucket<K, V>;

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            loop {
                if let Some(index) = self.current_group.lowest_set_bit() {
                    self.current_group = self.current_group.remove_lowest_bit();
                    self.items -= 1;
                    return Some(self.data.add(index));
                }

                if self.next_ctrl >= self.end {
                    debug_assert_eq!(self.items, 0);
                    return None;
                }

                self.current_group = Group::load_aligned(self.next_ctrl).match_full();
                self.data = self.data.add(Group::WIDTH);
                self.next_ctrl = self.next_ctrl.add(Group::WIDTH);
            }
        }
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items, Some(self.items))
    }
}

impl<'a, K: Hash + Eq, V> ExactSizeIterator for RawIter<'a, K, V> {}
impl<'a, K: Hash + Eq, V> FusedIterator for RawIter<'a, K, V> {}

struct RawIterPin<'a, K: Hash + Eq, V> {
    current_group: BitMask,
    data: *mut ArchivedBucket<K, V>,
    next_ctrl: *const u8,
    end: *const u8,
    items: usize,
    _phantom: PhantomData<(&'a K, &'a V)>,
}

impl<'a, K: Hash + Eq, V> RawIterPin<'a, K, V> {
    fn new(ctrl: *const u8, data: *mut ArchivedBucket<K, V>, buckets: usize, items: usize) -> Self {
        debug_assert_ne!(buckets, 0);
        debug_assert_eq!(ctrl as usize % Group::WIDTH, 0);
        unsafe {
            let end = ctrl.add(buckets);

            let current_group = Group::load_aligned(ctrl).match_full();
            let next_ctrl = ctrl.add(Group::WIDTH);

            Self {
                current_group,
                data,
                next_ctrl,
                end,
                items,
                _phantom: PhantomData,
            }
        }
    }
}

impl<'a, K: Hash + Eq, V> Iterator for RawIterPin<'a, K, V> {
    type Item = *mut ArchivedBucket<K, V>;

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            loop {
                if let Some(index) = self.current_group.lowest_set_bit() {
                    self.current_group = self.current_group.remove_lowest_bit();
                    self.items -= 1;
                    return Some(self.data.add(index));
                }

                if self.next_ctrl >= self.end {
                    debug_assert_eq!(self.items, 0);
                    return None;
                }

                self.current_group = Group::load_aligned(self.next_ctrl).match_full();
                self.data = self.data.add(Group::WIDTH);
                self.next_ctrl = self.next_ctrl.add(Group::WIDTH);
            }
        }
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.items, Some(self.items))
    }
}

impl<'a, K: Hash + Eq, V> ExactSizeIterator for RawIterPin<'a, K, V> {}
impl<'a, K: Hash + Eq, V> FusedIterator for RawIterPin<'a, K, V> {}

/// An iterator over the key-value pairs of a hash map.
#[repr(transparent)]
pub struct Iter<'a, K: Hash + Eq, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let bucket = &*x;
            (&bucket.key, &bucket.value)
        })
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for Iter<'_, K, V> {
    #[cfg_attr(feature = "inline_more", inline)]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K: Hash + Eq, V> FusedIterator for Iter<'_, K, V> {}

/// An iterator over the mutable key-value pairs of a hash map.
#[repr(transparent)]
pub struct IterPin<'a, K: Hash + Eq, V> {
    inner: RawIterPin<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for IterPin<'a, K, V> {
    type Item = (&'a K, Pin<&'a mut V>);

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let bucket = &mut *x;
            (&bucket.key, Pin::new_unchecked(&mut bucket.value))
        })
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for IterPin<'_, K, V> {
    #[cfg_attr(feature = "inline_more", inline)]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K: Hash + Eq, V> FusedIterator for IterPin<'_, K, V> {}

/// An iterator over the keys of a hash map.
#[repr(transparent)]
pub struct Keys<'a, K: Hash + Eq, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let bucket = &*x;
            &bucket.key
        })
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for Keys<'_, K, V> {
    #[cfg_attr(feature = "inline_more", inline)]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K: Hash + Eq, V> FusedIterator for Keys<'_, K, V> {}

/// An iterator over the values of a hash map.
#[repr(transparent)]
pub struct Values<'a, K: Hash + Eq, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let bucket = &*x;
            &bucket.value
        })
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for Values<'_, K, V> {
    #[cfg_attr(feature = "inline_more", inline)]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K: Hash + Eq, V> FusedIterator for Values<'_, K, V> {}

/// An iterator over the mutable values of a hash map.
#[repr(transparent)]
pub struct ValuesPin<'a, K: Hash + Eq, V> {
    inner: RawIterPin<'a, K, V>,
}

impl<'a, K: Hash + Eq, V> Iterator for ValuesPin<'a, K, V> {
    type Item = Pin<&'a mut V>;

    #[cfg_attr(feature = "inline_more", inline)]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|x| unsafe {
            let bucket = &mut *x;
            Pin::new_unchecked(&mut bucket.value)
        })
    }

    #[cfg_attr(feature = "inline_more", inline)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<K: Hash + Eq, V> ExactSizeIterator for ValuesPin<'_, K, V> {
    #[cfg_attr(feature = "inline_more", inline)]
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<K: Hash + Eq, V> FusedIterator for ValuesPin<'_, K, V> {}

/// An archived `HashSet`. This is a wrapper around a hash map with the same key
/// and a value of `()`.
#[derive(Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct ArchivedHashSet<K: Hash + Eq>(ArchivedHashMap<K, ()>);

#[doc(hidden)]
pub struct ArchivedHashSetResolver(ArchivedHashMapResolver);

impl<K: Hash + Eq> ArchivedHashSet<K> {
    /// Gets the number of items in the hash set.
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
        Ok(ArchivedHashSetResolver(ArchivedHashMap::archive_from_iter(
            self.iter().map(|x| (x, &())),
            self.len(),
            writer,
        )?))
    }
}
