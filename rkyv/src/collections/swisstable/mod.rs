//! An archived hash map implementation based on Google's high-performance
//! SwissTable hash map.
//!
//! Notable differences from other implementations:
//!
//! - The number of control bytes is rounded up to a maximum group width (16)
//!   instead of the next power of two. This reduces the number of empty buckets
//!   on the wire. Since this collection is immutable after writing, we'll never
//!   benefit from having more buckets than we need.
//! - Because the bucket count is not a power of two, the triangular probing
//!   sequence simply skips any indices larger than the actual size of the
//!   buckets array.
//! - Instead of the final control bytes always being marked EMPTY, the last
//!   control bytes repeat the first few. This helps reduce the number of
//!   lookups when probing at the end of the control bytes.
//! - Because the available SIMD group width may be less than the maximum group
//!   width, each probe reads N groups before striding where N is the maximum
//!   group width divided by the SIMD group width.

use core::{
    alloc::Layout,
    borrow::Borrow,
    fmt,
    hash::Hash,
    iter::FusedIterator,
    marker::PhantomData,
    mem::size_of,
    ops::Index,
    pin::Pin,
    ptr::{self, null_mut, NonNull},
    slice,
};

use rancor::{fail, Error, Fallible, OptionExt, Panic, ResultExt as _};

use crate::{
    primitive::ArchivedUsize,
    ser::{Allocator, Writer, WriterExt},
    simd::{Bitmask, Group, MAX_GROUP_WIDTH},
    util::ScratchVec,
    Archive as _, RawRelPtr, Serialize,
};

#[cfg_attr(feature = "stable_layout", repr(C))]
struct Entry<K, V> {
    key: K,
    value: V,
}

/// An archived SwissTable hash map.
#[cfg_attr(feature = "stable_layout", repr(C))]
#[derive(Debug)]
pub struct ArchivedSwissTable<K, V> {
    ptr: RawRelPtr,
    len: ArchivedUsize,
    cap: ArchivedUsize,
    _phantom: PhantomData<(K, V)>,
}

#[inline]
fn h1(hash: u64) -> usize {
    hash as usize
}

#[inline]
fn h2(hash: u64) -> u8 {
    (hash >> 57) as u8
}

struct ProbeSeq {
    pos: usize,
    stride: usize,
}

impl ProbeSeq {
    #[inline]
    fn next_group(&mut self) {
        self.pos += Group::WIDTH;
    }

    #[inline]
    fn move_next(&mut self, bucket_mask: usize) {
        loop {
            self.pos += self.stride;
            self.pos &= bucket_mask;
            self.stride += MAX_GROUP_WIDTH;
        }
    }
}

impl<K, V> ArchivedSwissTable<K, V> {
    fn hash_value<Q>(value: &Q) -> u64
    where
        Q: Hash + ?Sized,
    {
        use core::hash::Hasher;
        use seahash::SeaHasher;

        // TODO: switch hasher / pick nothing-up-my-sleeve numbers for initial
        // state seeds
        let mut state = SeaHasher::with_seeds(
            0x00000000_00000000,
            0x00000000_00000000,
            0x00000000_00000000,
            0x00000000_00000000,
        );
        value.hash(&mut state);
        state.finish()
    }

    fn probe_seq(hash: u64, capacity: usize) -> ProbeSeq {
        ProbeSeq {
            pos: h1(hash) % capacity,
            stride: 0,
        }
    }

    #[inline]
    unsafe fn control(&self, index: usize) -> *mut u8 {
        self.ptr.as_ptr().cast::<u8>().add(index)
    }

    #[inline]
    unsafe fn bucket(&self, index: usize) -> NonNull<Entry<K, V>> {
        unsafe {
            NonNull::new_unchecked(
                self.ptr.as_ptr().cast::<Entry<K, V>>().sub(index + 1),
            )
        }
    }

    #[inline]
    fn bucket_mask(capacity: usize) -> usize {
        capacity.checked_next_power_of_two().unwrap() - 1
    }

    #[inline(always)]
    fn get_entry<C>(&self, hash: u64, cmp: C) -> Option<NonNull<Entry<K, V>>>
    where
        C: Fn(&K) -> bool,
    {
        if self.len.to_native() == 0 {
            return None;
        }

        let h2_hash = h2(hash);
        let mut probe_seq = Self::probe_seq(hash, self.capacity());

        let capacity = self.capacity();
        let bucket_mask = Self::bucket_mask(capacity);

        loop {
            for _ in 0..MAX_GROUP_WIDTH / Group::WIDTH {
                let group = unsafe { Group::read(self.control(probe_seq.pos)) };

                for bit in group.match_byte(h2_hash) {
                    let index = (probe_seq.pos + bit) % capacity;
                    let bucket_ptr = unsafe { self.bucket(index) };
                    let bucket = unsafe { bucket_ptr.as_ref() };

                    // TODO: likely
                    if cmp(&bucket.key) {
                        return Some(bucket_ptr);
                    }
                }

                // TODO: likely
                if group.match_empty().any_bit_set() {
                    return None;
                }

                probe_seq.next_group();
            }

            loop {
                probe_seq.move_next(bucket_mask);
                if probe_seq.pos < self.capacity() {
                    break;
                }
            }
        }
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let ptr =
            self.get_entry(Self::hash_value(key), |k| k.borrow() == key)?;
        let entry = unsafe { ptr.as_ref() };
        Some((&entry.key, &entry.value))
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

    /// Returns the mutable key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value_mut<Q>(
        self: Pin<&mut Self>,
        key: &Q,
    ) -> Option<(&K, &mut V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let mut ptr =
            self.get_entry(Self::hash_value(key), |k| k.borrow() == key)?;
        let entry = unsafe { ptr.as_mut() };
        Some((&entry.key, &mut entry.value))
    }

    /// Returns a mutable reference to the value corresponding to the supplied key.
    #[inline]
    pub fn get_mut<Q>(self: Pin<&mut Self>, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        Some(self.get_key_value_mut(key)?.1)
    }

    /// Returns the key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value_with<Q, C>(&self, key: &Q, cmp: C) -> Option<(&K, &V)>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&K, &Q) -> bool,
    {
        let ptr = self.get_entry(Self::hash_value(key), |k| cmp(k, key))?;
        let entry = unsafe { ptr.as_ref() };
        Some((&entry.key, &entry.value))
    }

    /// Returns a reference to the value corresponding to the supplied key.
    #[inline]
    pub fn get_with<Q, C>(&self, key: &Q, cmp: C) -> Option<&V>
    where
        Q: Hash + Eq + ?Sized,
        C: Fn(&K, &Q) -> bool,
    {
        Some(self.get_key_value_with(key, cmp)?.1)
    }

    /// Returns the mutable key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_key_value_mut_with<Q, C>(
        self: Pin<&mut Self>,
        key: &Q,
        cmp: C,
    ) -> Option<(&K, &mut V)>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        C: Fn(&K, &Q) -> bool,
    {
        let mut ptr = self.get_entry(Self::hash_value(key), |k| cmp(k, key))?;
        let entry = unsafe { ptr.as_mut() };
        Some((&entry.key, &mut entry.value))
    }

    /// Returns a mutable reference to the value corresponding to the supplied key.
    #[inline]
    pub fn get_mut_with<Q, C>(
        self: Pin<&mut Self>,
        key: &Q,
        cmp: C,
    ) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
        C: Fn(&K, &Q) -> bool,
    {
        Some(self.get_key_value_mut_with(key, cmp)?.1)
    }

    /// Returns whether the SwissTable contains the given key.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.get(key).is_some()
    }

    /// Returns whether the SwissTable is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len.to_native() == 0
    }

    /// Returns the number of elements in the SwissTable.
    #[inline]
    pub fn len(&self) -> usize {
        self.len.to_native() as usize
    }

    /// Returns the total capacity of the SwissTable.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap.to_native() as usize
    }

    fn raw_iter(&self) -> RawIter<K, V> {
        if self.is_empty() {
            RawIter {
                current_mask: Bitmask::EMPTY,
                entries: null_mut(),
                next_group: null_mut(),
                items_left: 0,
            }
        } else {
            RawIter {
                current_mask: unsafe {
                    Group::read(self.control(0)).match_full()
                },
                entries: unsafe { self.ptr.as_ptr().cast() },
                next_group: unsafe { self.control(Group::WIDTH) },
                items_left: self.len(),
            }
        }
    }

    /// Returns an iterator over the key-value entries in the SwissTable.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            raw: self.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the mutable key-value entries in the
    /// SwissTable.
    #[inline]
    pub fn iter_mut(self: Pin<&mut Self>) -> IterMut<'_, K, V> {
        IterMut {
            raw: self.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the keys in the SwissTable.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            raw: self.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the values in the SwissTable.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            raw: self.raw_iter(),
            _phantom: PhantomData,
        }
    }

    /// Returns an iterator over the mutable values in the SwissTable.
    #[inline]
    pub fn values_mut(self: Pin<&mut Self>) -> ValuesMut<'_, K, V> {
        ValuesMut {
            raw: self.raw_iter(),
            _phantom: PhantomData,
        }
    }

    #[inline]
    fn capacity_from_len<E: Error>(
        len: usize,
        load_factor: (usize, usize),
    ) -> Result<usize, E> {
        Ok(len
            .checked_mul(load_factor.1)
            .into_trace("overflow while adjusting capacity")?
            / load_factor.0)
    }

    #[inline]
    fn control_count<E: Error>(capacity: usize) -> Result<usize, E> {
        capacity
            .checked_next_multiple_of(MAX_GROUP_WIDTH)
            .into_trace(
                "overflow while calculating buckets from adjusted capacity",
            )
    }

    /// Serializes an iterator of key-value pairs as a hash map.
    pub fn serialize_from_iter<'a, KU, VU, I, S>(
        mut iter: I,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<SwissTableResolver, S::Error>
    where
        KU: 'a + Serialize<S, Archived = K> + Hash + Eq,
        VU: 'a + Serialize<S, Archived = V>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Error,
        I: Clone + ExactSizeIterator<Item = (&'a KU, &'a VU)>,
    {
        // TODO: error if load_factor.0 is greater than load_factor.1

        #[derive(Debug)]
        struct IteratorLengthMismatch {
            expected: usize,
            actual: usize,
        }

        impl fmt::Display for IteratorLengthMismatch {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "iterator claimed that it contained {} elements, but yielded {} items during iteration",
                    self.expected,
                    self.actual,
                )
            }
        }

        #[cfg(feature = "std")]
        impl std::error::Error for IteratorLengthMismatch {}

        let len = iter.len();

        if len == 0 {
            if iter.next().is_some() {
                fail!(IteratorLengthMismatch {
                    expected: 0,
                    actual: 1 + iter.count(),
                });
            }

            return Ok(SwissTableResolver { pos: 0 });
        }

        // Serialize all items
        let mut resolvers = unsafe { ScratchVec::new(serializer, len)? };
        for (key, value) in iter.clone() {
            if resolvers.len() == len {
                fail!(IteratorLengthMismatch {
                    expected: len,
                    actual: len + iter.count(),
                });
            }

            resolvers.push((
                key.serialize(serializer)?,
                value.serialize(serializer)?,
            ));
        }

        // Allocate scratch space for the SwissTable storage
        let capacity = Self::capacity_from_len(len, load_factor)?;
        let control_count = Self::control_count(capacity)?;

        let buckets_layout =
            Layout::array::<Entry<K, V>>(capacity).into_error()?;
        let control_layout = unsafe {
            Layout::from_size_align_unchecked(control_count, MAX_GROUP_WIDTH)
        };
        let (layout, control_offset) =
            buckets_layout.extend(control_layout).into_error()?;

        let alloc = unsafe { serializer.push_alloc(layout)?.cast::<u8>() };

        // Initialize all non-control bytes to zero
        unsafe {
            ptr::write_bytes(alloc.as_ptr(), 0, control_offset);
        }

        let ptr = unsafe { alloc.as_ptr().add(control_offset) };

        // Initialize all control bytes to EMPTY (0xFF)
        unsafe {
            ptr::write_bytes(ptr, 0xff, control_count);
        }

        let bucket_mask = Self::bucket_mask(capacity);

        let pos = serializer.align(layout.align())?;

        for ((key, value), (key_resolver, value_resolver)) in
            iter.zip(resolvers.drain(..))
        {
            let hash = Self::hash_value(key);
            let h2_hash = h2(hash);
            let mut probe_seq = Self::probe_seq(hash, capacity);

            'insert: loop {
                for _ in 0..MAX_GROUP_WIDTH / Group::WIDTH {
                    let group = unsafe { Group::read(ptr.add(probe_seq.pos)) };

                    if let Some(bit) = group.match_empty().lowest_set_bit() {
                        let control_index = (probe_seq.pos + bit) & bucket_mask;
                        let index = control_index % capacity;

                        // Update control byte(s)
                        unsafe {
                            ptr.add(control_index).write(h2_hash);
                            ptr.add(index).write(h2_hash);
                        }

                        let entry_pos = pos + control_offset
                            - (index + 1) * size_of::<Entry<K, V>>();
                        let out = unsafe {
                            alloc.as_ptr().add(entry_pos).cast::<Entry<K, V>>()
                        };
                        let (fp, fo) = out_field!(out.key);
                        unsafe {
                            key.resolve(pos + fp, key_resolver, fo);
                        }
                        let (fp, fo) = out_field!(out.value);
                        unsafe {
                            value.resolve(pos + fp, value_resolver, fo);
                        }

                        break 'insert;
                    }

                    probe_seq.next_group();
                }

                loop {
                    probe_seq.move_next(bucket_mask);
                    if probe_seq.pos < capacity {
                        break;
                    }
                }
            }
        }

        // Write out-of-line data
        let slice =
            unsafe { slice::from_raw_parts(alloc.as_ptr(), layout.size()) };
        serializer.write(slice)?;

        unsafe {
            serializer.pop_alloc(alloc, layout)?;
        }

        unsafe {
            resolvers.free(serializer)?;
        }

        Ok(SwissTableResolver {
            pos: pos + control_offset,
        })
    }

    /// Resolves an archived hash map from a given length and parameters.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that properly aligned and valid for writes.
    pub unsafe fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        pos: usize,
        resolver: SwissTableResolver,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.ptr);
        RawRelPtr::emplace(pos + fp, resolver.pos, fo);

        let (fp, fo) = out_field!(out.len);
        len.resolve(pos + fp, (), fo);

        let (fp, fo) = out_field!(out.cap);
        let capacity =
            Self::capacity_from_len::<Panic>(len, load_factor).always_ok();
        capacity.resolve(pos + fp, (), fo);

        // PhantomData doesn't need to be initialized
    }
}

impl<K, V, Q> Index<&Q> for ArchivedSwissTable<K, V>
where
    K: Eq + Hash + Borrow<Q>,
    Q: Eq + Hash + ?Sized,
{
    type Output = V;

    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).unwrap()
    }
}

/// The resolver for archived [SwissTables](ArchivedSwissTable).
pub struct SwissTableResolver {
    pos: usize,
}

struct RawIter<K, V> {
    current_mask: Bitmask,
    entries: *mut Entry<K, V>,
    next_group: *const u8,
    items_left: usize,
}

unsafe impl<K, V> Send for RawIter<K, V> {}
unsafe impl<K, V> Sync for RawIter<K, V> {}

impl<K, V> Iterator for RawIter<K, V> {
    type Item = *mut Entry<K, V>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.items_left == 0 {
            None
        } else {
            loop {
                if let Some(bit) = self.current_mask.lowest_set_bit() {
                    self.current_mask.remove_lowest_bit();
                    let entry = unsafe { self.entries.sub(bit + 1) };
                    self.items_left -= 1;
                    break Some(entry);
                } else {
                    self.current_mask =
                        unsafe { Group::read(self.next_group).match_full() };
                    self.entries = unsafe { self.entries.sub(Group::WIDTH) };
                    self.next_group =
                        unsafe { self.next_group.add(Group::WIDTH) };
                }
            }
        }
    }
}

/// An iterator over the key-value pairs of a SwissTable.
pub struct Iter<'a, K, V> {
    raw: RawIter<K, V>,
    _phantom: PhantomData<&'a ArchivedSwissTable<K, V>>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { &*entry };
            (&entry.key, &entry.value)
        })
    }
}

impl<K, V> ExactSizeIterator for Iter<'_, K, V> {
    fn len(&self) -> usize {
        self.raw.items_left
    }
}

impl<K, V> FusedIterator for Iter<'_, K, V> {}

/// An iterator over the mutable key-value pairs of a SwissTable.
pub struct IterMut<'a, K, V> {
    raw: RawIter<K, V>,
    _phantom: PhantomData<&'a ArchivedSwissTable<K, V>>,
}

impl<'a, K, V> Iterator for IterMut<'a, K, V> {
    type Item = (&'a K, Pin<&'a mut V>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { &mut *entry };
            let value = unsafe { Pin::new_unchecked(&mut entry.value) };
            (&entry.key, value)
        })
    }
}

impl<K, V> ExactSizeIterator for IterMut<'_, K, V> {
    fn len(&self) -> usize {
        self.raw.items_left
    }
}

impl<K, V> FusedIterator for IterMut<'_, K, V> {}

/// An iterator over the keys of a SwissTable.
pub struct Keys<'a, K, V> {
    raw: RawIter<K, V>,
    _phantom: PhantomData<&'a ArchivedSwissTable<K, V>>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { &*entry };
            &entry.key
        })
    }
}

impl<K, V> ExactSizeIterator for Keys<'_, K, V> {
    fn len(&self) -> usize {
        self.raw.items_left
    }
}

impl<K, V> FusedIterator for Keys<'_, K, V> {}

/// An iterator over the values of a SwissTable.
pub struct Values<'a, K, V> {
    raw: RawIter<K, V>,
    _phantom: PhantomData<&'a ArchivedSwissTable<K, V>>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { &*entry };
            &entry.value
        })
    }
}

impl<K, V> ExactSizeIterator for Values<'_, K, V> {
    fn len(&self) -> usize {
        self.raw.items_left
    }
}

impl<K, V> FusedIterator for Values<'_, K, V> {}

/// An iterator over the mutable values of a SwissTable.
pub struct ValuesMut<'a, K, V> {
    raw: RawIter<K, V>,
    _phantom: PhantomData<&'a ArchivedSwissTable<K, V>>,
}

impl<'a, K, V> Iterator for ValuesMut<'a, K, V> {
    type Item = Pin<&'a mut V>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.raw.next().map(|entry| {
            let entry = unsafe { &mut *entry };
            unsafe { Pin::new_unchecked(&mut entry.value) }
        })
    }
}

impl<K, V> ExactSizeIterator for ValuesMut<'_, K, V> {
    fn len(&self) -> usize {
        self.raw.items_left
    }
}

impl<K, V> FusedIterator for ValuesMut<'_, K, V> {}
