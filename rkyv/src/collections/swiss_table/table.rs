//! An archived hash table implementation based on Google's high-performance
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
    fmt,
    marker::PhantomData,
    mem::size_of,
    pin::Pin,
    ptr::{self, null, NonNull},
    slice,
};

use rancor::{fail, Error, Fallible, OptionExt, Panic, ResultExt as _};

use crate::{
    primitive::ArchivedUsize,
    ser::{Allocator, Writer, WriterExt},
    simd::{Bitmask, Group, MAX_GROUP_WIDTH},
    util::ScratchVec,
    Archive as _, Portable, RawRelPtr, Serialize,
};

/// A low-level archived SwissTable hash table with explicit hashing.
#[derive(Portable)]
#[archive(crate)]
#[repr(C)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    check_bytes(verify)
)]
pub struct ArchivedHashTable<T> {
    ptr: RawRelPtr,
    len: ArchivedUsize,
    cap: ArchivedUsize,
    _phantom: PhantomData<T>,
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

impl<T> ArchivedHashTable<T> {
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
    unsafe fn bucket(&self, index: usize) -> NonNull<T> {
        unsafe {
            NonNull::new_unchecked(self.ptr.as_ptr().cast::<T>().sub(index + 1))
        }
    }

    #[inline]
    fn bucket_mask(capacity: usize) -> usize {
        capacity.checked_next_power_of_two().unwrap() - 1
    }

    #[inline(always)]
    fn get_entry<C>(&self, hash: u64, cmp: C) -> Option<NonNull<T>>
    where
        C: Fn(&T) -> bool,
    {
        if self.len.to_native() == 0 {
            return None;
        }

        let h2_hash = h2(hash);
        let mut probe_seq = Self::probe_seq(hash, self.capacity());

        let capacity = self.capacity();
        let bucket_mask = Self::bucket_mask(capacity);

        loop {
            let mut any_empty = false;

            for _ in 0..MAX_GROUP_WIDTH / Group::WIDTH {
                let group = unsafe { Group::read(self.control(probe_seq.pos)) };

                for bit in group.match_byte(h2_hash) {
                    let index = (probe_seq.pos + bit) % capacity;
                    let bucket_ptr = unsafe { self.bucket(index) };
                    let bucket = unsafe { bucket_ptr.as_ref() };

                    // TODO: likely
                    if cmp(bucket) {
                        return Some(bucket_ptr);
                    }
                }

                // TODO: likely
                any_empty = any_empty || group.match_empty().any_bit_set();

                probe_seq.next_group();
            }

            if any_empty {
                return None;
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
    pub fn get_with<C>(&self, hash: u64, cmp: C) -> Option<&T>
    where
        C: Fn(&T) -> bool,
    {
        let ptr = self.get_entry(hash, |e| cmp(e))?;
        Some(unsafe { ptr.as_ref() })
    }

    /// Returns the mutable key-value pair corresponding to the supplied key.
    #[inline]
    pub fn get_with_mut<C>(
        self: Pin<&mut Self>,
        hash: u64,
        cmp: C,
    ) -> Option<Pin<&mut T>>
    where
        C: Fn(&T) -> bool,
    {
        let mut ptr = self.get_entry(hash, |e| cmp(e))?;
        Some(unsafe { Pin::new_unchecked(ptr.as_mut()) })
    }

    /// Returns whether the hash table is empty.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len.to_native() == 0
    }

    /// Returns the number of elements in the hash table.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len.to_native() as usize
    }

    /// Returns the total capacity of the hash table.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap.to_native() as usize
    }

    fn control_iter(&self) -> ControlIter {
        ControlIter {
            current_mask: unsafe { Group::read(self.control(0)).match_full() },
            next_group: unsafe { self.control(Group::WIDTH) },
        }
    }

    /// Returns an iterator over the entry pointers in the hash table.
    pub fn raw_iter(&self) -> RawIter<T> {
        if self.is_empty() {
            RawIter {
                controls: ControlIter::none(),
                entries: NonNull::dangling(),
                items_left: 0,
            }
        } else {
            RawIter {
                controls: self.control_iter(),
                entries: unsafe {
                    NonNull::new_unchecked(self.ptr.as_ptr().cast())
                },
                items_left: self.len(),
            }
        }
    }

    #[inline]
    fn capacity_from_len<E: Error>(
        len: usize,
        load_factor: (usize, usize),
    ) -> Result<usize, E> {
        Ok(usize::max(
            len.checked_mul(load_factor.1)
                .into_trace("overflow while adjusting capacity")?
                / load_factor.0,
            len + 1,
        ))
    }

    #[inline]
    fn control_count<E: Error>(capacity: usize) -> Result<usize, E> {
        capacity.checked_add(MAX_GROUP_WIDTH - 1).into_trace(
            "overflow while calculating buckets from adjusted capacity",
        )
    }

    fn memory_layout<E: Error>(
        capacity: usize,
        control_count: usize,
    ) -> Result<(Layout, usize), E> {
        let buckets_layout = Layout::array::<T>(capacity).into_error()?;
        let control_layout = Layout::array::<u8>(control_count).into_error()?;
        buckets_layout.extend(control_layout).into_error()
    }

    /// Serializes an iterator of items as a hash table.
    pub fn serialize_from_iter<I, H, S>(
        items: I,
        hashes: H,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<HashTableResolver, S::Error>
    where
        I: Clone + ExactSizeIterator,
        I::Item: Serialize<S, Archived = T>,
        H: ExactSizeIterator<Item = u64>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Error,
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

        let len = items.len();

        if len == 0 {
            let count = items.count();
            if count != 0 {
                fail!(IteratorLengthMismatch {
                    expected: 0,
                    actual: count,
                });
            }

            return Ok(HashTableResolver { pos: 0 });
        }

        // Serialize all items
        let mut resolvers = unsafe { ScratchVec::new(serializer, len)? };
        for i in items.clone() {
            if resolvers.len() == len {
                fail!(IteratorLengthMismatch {
                    expected: len,
                    actual: len + items.count(),
                });
            }

            resolvers.push(i.serialize(serializer)?);
        }

        // Allocate scratch space for the hash table storage
        let capacity = Self::capacity_from_len(len, load_factor)?;
        let control_count = Self::control_count(capacity)?;

        let (layout, control_offset) =
            Self::memory_layout(capacity, control_count)?;

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

        for ((i, resolver), hash) in items.zip(resolvers.drain(..)).zip(hashes)
        {
            let h2_hash = h2(hash);
            let mut probe_seq = Self::probe_seq(hash, capacity);

            'insert: loop {
                for _ in 0..MAX_GROUP_WIDTH / Group::WIDTH {
                    let group = unsafe { Group::read(ptr.add(probe_seq.pos)) };

                    if let Some(bit) = group.match_empty().lowest_set_bit() {
                        let index = (probe_seq.pos + bit) % capacity;

                        // Update control byte
                        unsafe {
                            ptr.add(index).write(h2_hash);
                        }
                        // If it's near the end of the group, update the
                        // wraparound control byte
                        if index < control_count - capacity {
                            unsafe {
                                ptr.add(capacity + index).write(h2_hash);
                            }
                        }

                        let entry_offset =
                            control_offset - (index + 1) * size_of::<T>();
                        let out = unsafe {
                            alloc.as_ptr().add(entry_offset).cast::<T>()
                        };
                        unsafe {
                            i.resolve(pos + entry_offset, resolver, out);
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

        Ok(HashTableResolver {
            pos: pos + control_offset,
        })
    }

    /// Resolves an archived hash table from a given length and parameters.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that properly aligned and valid for writes.
    pub unsafe fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        pos: usize,
        resolver: HashTableResolver,
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

/// The resolver for [`ArchivedHashTable`].
pub struct HashTableResolver {
    pos: usize,
}

struct ControlIter {
    current_mask: Bitmask,
    next_group: *const u8,
}

unsafe impl Send for ControlIter {}
unsafe impl Sync for ControlIter {}

impl ControlIter {
    fn none() -> Self {
        Self {
            current_mask: Bitmask::EMPTY,
            next_group: null(),
        }
    }

    #[inline]
    fn next_full(&mut self) -> Option<usize> {
        let bit = self.current_mask.lowest_set_bit()?;
        self.current_mask = self.current_mask.remove_lowest_bit();
        Some(bit)
    }

    #[inline]
    fn move_next(&mut self) {
        self.current_mask =
            unsafe { Group::read(self.next_group).match_full() };
        self.next_group = unsafe { self.next_group.add(Group::WIDTH) };
    }
}

/// An iterator over the entry pointers of an [`ArchivedHashTable`].
pub struct RawIter<T> {
    controls: ControlIter,
    entries: NonNull<T>,
    items_left: usize,
}

impl<T> Iterator for RawIter<T> {
    type Item = NonNull<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.items_left == 0 {
            None
        } else {
            let bit = loop {
                if let Some(bit) = self.controls.next_full() {
                    break bit;
                }
                self.controls.move_next();
                self.entries = unsafe {
                    NonNull::new_unchecked(
                        self.entries.as_ptr().sub(Group::WIDTH),
                    )
                };
            };
            self.items_left -= 1;
            let entry = unsafe {
                NonNull::new_unchecked(self.entries.as_ptr().sub(bit + 1))
            };
            Some(entry)
        }
    }
}

impl<T> ExactSizeIterator for RawIter<T> {
    #[inline]
    fn len(&self) -> usize {
        self.items_left
    }
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::fmt;

    use bytecheck::{CheckBytes, Verify};
    use rancor::{fail, Error, Fallible};

    use super::ArchivedHashTable;
    use crate::{
        simd::Group,
        validation::{ArchiveContext, ArchiveContextExt},
    };

    #[derive(Debug)]
    struct InvalidLength {
        len: usize,
        cap: usize,
    }

    impl fmt::Display for InvalidLength {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "hash table length must be strictly less than its capacity (length: {}, capacity: {})",
                self.len,
                self.cap,
            )
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for InvalidLength {}

    #[derive(Debug)]
    struct UnwrappedControlByte {
        index: usize,
    }

    impl fmt::Display for UnwrappedControlByte {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "unwrapped control byte at index {}", self.index,)
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for UnwrappedControlByte {}

    unsafe impl<C, T> Verify<C> for ArchivedHashTable<T>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Error,
        T: CheckBytes<C>,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let len = self.len();
            let cap = self.capacity();

            if len == 0 && cap == 0 {
                return Ok(());
            }

            if self.len() >= cap {
                fail!(InvalidLength { len, cap });
            }

            // Check memory allocation
            let control_count = Self::control_count(cap)?;
            let (layout, control_offset) =
                Self::memory_layout(cap, control_count)?;
            let ptr = self
                .ptr
                .as_ptr_wrapping()
                .cast::<u8>()
                .wrapping_sub(control_offset);
            context.check_subtree_ptr(ptr, &layout)?;

            let range = unsafe { context.push_prefix_subtree(ptr)? };

            // Check each non-empty bucket
            let mut controls = self.control_iter();
            let mut base_index = 0;
            'outer: while base_index < cap {
                while let Some(bit) = controls.next_full() {
                    let index = base_index + bit;
                    if index >= cap {
                        break 'outer;
                    }

                    unsafe {
                        T::check_bytes(self.bucket(index).as_ptr(), context)?;
                    }
                }

                controls.move_next();
                base_index += Group::WIDTH;
            }

            // Verify that wrapped bytes are set correctly
            for i in cap..usize::min(2 * cap, control_count) {
                let byte = unsafe { *self.control(i) };
                let wrapped = unsafe { *self.control(i % cap) };
                if wrapped != byte {
                    fail!(UnwrappedControlByte { index: i })
                }
            }

            unsafe {
                context.pop_subtree_range(range)?;
            }

            Ok(())
        }
    }
}
