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
    borrow::Borrow,
    error::Error,
    fmt,
    marker::PhantomData,
    mem::{size_of, MaybeUninit},
    ptr::{self, null, NonNull},
    slice::from_raw_parts,
};

use munge::munge;
use rancor::{fail, Fallible, ResultExt as _, Source};

use crate::{
    collections::util::IteratorLengthMismatch,
    primitive::{ArchivedUsize, FixedUsize},
    seal::Seal,
    ser::{Allocator, Writer, WriterExt},
    simd::{Bitmask, Group, MAX_GROUP_WIDTH},
    util::SerVec,
    Archive as _, Place, Portable, RawRelPtr, Serialize,
};

/// A low-level archived SwissTable hash table with explicit hashing.
#[derive(Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[rkyv(crate)]
#[repr(C)]
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
    fn move_next(&mut self, bucket_mask: usize) {
        self.stride += MAX_GROUP_WIDTH;
        self.pos += self.stride;
        self.pos &= bucket_mask;
    }
}

impl<T> ArchivedHashTable<T> {
    fn probe_seq(hash: u64, capacity: usize) -> ProbeSeq {
        ProbeSeq {
            pos: h1(hash) % capacity,
            stride: 0,
        }
    }

    /// # Safety
    ///
    /// - `this` must point to a valid `ArchivedHashTable`
    /// - `index` must be less than `len()`
    unsafe fn control_raw(this: *mut Self, index: usize) -> *const u8 {
        debug_assert!(unsafe { !(*this).is_empty() });

        // SAFETY: As an invariant of `ArchivedHashTable`, if `self` is not
        // empty then `self.ptr` is a valid relative pointer. Since `index` is
        // at least 0 and strictly less than `len()`, this table must not be
        // empty.
        let ptr =
            unsafe { RawRelPtr::as_ptr_raw(ptr::addr_of_mut!((*this).ptr)) };
        // SAFETY: The caller has guaranteed that `index` is less than `len()`,
        // and the first `len()` bytes following `ptr` are the control bytes of
        // the hash table.
        unsafe { ptr.cast::<u8>().add(index) }
    }

    /// # Safety
    ///
    /// - `this` must point to a valid `ArchivedHashTable`
    /// - `index` must be less than `len()`
    unsafe fn bucket_raw(this: *mut Self, index: usize) -> NonNull<T> {
        unsafe {
            NonNull::new_unchecked(
                RawRelPtr::as_ptr_raw(ptr::addr_of_mut!((*this).ptr))
                    .cast::<T>()
                    .sub(index + 1),
            )
        }
    }

    fn bucket_mask(capacity: usize) -> usize {
        capacity.checked_next_power_of_two().unwrap() - 1
    }

    /// # Safety
    ///
    /// `this` must point to a valid `ArchivedHashTable`
    unsafe fn get_entry_raw<C>(
        this: *mut Self,
        hash: u64,
        cmp: C,
    ) -> Option<NonNull<T>>
    where
        C: Fn(&T) -> bool,
    {
        let is_empty = unsafe { (*this).is_empty() };
        if is_empty {
            return None;
        }

        let capacity = unsafe { (*this).capacity() };
        let probe_cap = Self::probe_cap(capacity);
        let control_count = Self::control_count(probe_cap);

        let h2_hash = h2(hash);
        let mut probe_seq = Self::probe_seq(hash, capacity);

        let bucket_mask = Self::bucket_mask(control_count);

        loop {
            let mut any_empty = false;

            for i in 0..MAX_GROUP_WIDTH / Group::WIDTH {
                let pos = probe_seq.pos + i * Group::WIDTH;

                let group =
                    unsafe { Group::read(Self::control_raw(this, pos)) };

                for bit in group.match_byte(h2_hash) {
                    let index = (pos + bit) % capacity;
                    let bucket_ptr = unsafe { Self::bucket_raw(this, index) };
                    let bucket = unsafe { bucket_ptr.as_ref() };

                    // Opt: These can be marked as likely true on nightly.
                    if cmp(bucket) {
                        return Some(bucket_ptr);
                    }
                }

                // Opt: These can be marked as likely true on nightly.
                any_empty = any_empty || group.match_empty().any_bit_set();
            }

            if any_empty {
                return None;
            }

            loop {
                probe_seq.move_next(bucket_mask);
                if probe_seq.pos < probe_cap {
                    break;
                }
            }
        }
    }

    /// Returns the key-value pair corresponding to the supplied key.
    pub fn get_with<C>(&self, hash: u64, cmp: C) -> Option<&T>
    where
        C: Fn(&T) -> bool,
    {
        let this = (self as *const Self).cast_mut();
        let ptr = unsafe { Self::get_entry_raw(this, hash, |e| cmp(e))? };
        Some(unsafe { ptr.as_ref() })
    }

    /// Returns the mutable key-value pair corresponding to the supplied key.
    pub fn get_seal_with<C>(
        this: Seal<'_, Self>,
        hash: u64,
        cmp: C,
    ) -> Option<Seal<'_, T>>
    where
        C: Fn(&T) -> bool,
    {
        let mut ptr = unsafe {
            Self::get_entry_raw(this.unseal_unchecked(), hash, |e| cmp(e))?
        };
        Some(Seal::new(unsafe { ptr.as_mut() }))
    }

    /// Returns whether the hash table is empty.
    pub const fn is_empty(&self) -> bool {
        self.len.to_native() == 0
    }

    /// Returns the number of elements in the hash table.
    pub const fn len(&self) -> usize {
        self.len.to_native() as usize
    }

    /// Returns the total capacity of the hash table.
    pub fn capacity(&self) -> usize {
        self.cap.to_native() as usize
    }

    /// # Safety
    ///
    /// This hash table must not be empty.
    unsafe fn control_iter(this: *mut Self) -> ControlIter {
        ControlIter {
            current_mask: unsafe {
                Group::read(Self::control_raw(this, 0)).match_full()
            },
            next_group: unsafe { Self::control_raw(this, Group::WIDTH) },
        }
    }

    /// Returns an iterator over the entry pointers in the hash table.
    pub fn raw_iter(&self) -> RawIter<T> {
        if self.is_empty() {
            RawIter::empty()
        } else {
            let this = (self as *const Self).cast_mut();
            RawIter {
                // SAFETY: We have checked that `self` is not empty.
                controls: unsafe { Self::control_iter(this) },
                entries: unsafe {
                    NonNull::new_unchecked(self.ptr.as_ptr().cast_mut().cast())
                },
                items_left: self.len(),
            }
        }
    }

    /// Returns a sealed iterator over the entry pointers in the hash table.
    pub fn raw_iter_seal(mut this: Seal<'_, Self>) -> RawIter<T> {
        if this.is_empty() {
            RawIter::empty()
        } else {
            // SAFETY: We have checked that `this` is not empty.
            let controls =
                unsafe { Self::control_iter(this.as_mut().unseal_unchecked()) };
            let items_left = this.len();
            munge!(let Self { ptr, .. } = this);
            RawIter {
                controls,
                entries: unsafe {
                    NonNull::new_unchecked(RawRelPtr::as_mut_ptr(ptr).cast())
                },
                items_left,
            }
        }
    }

    fn capacity_from_len(len: usize, load_factor: (usize, usize)) -> usize {
        if len == 0 {
            0
        } else {
            usize::max(len * load_factor.1 / load_factor.0, len + 1)
        }
    }

    fn probe_cap(capacity: usize) -> usize {
        capacity.next_multiple_of(MAX_GROUP_WIDTH)
    }

    fn control_count(probe_cap: usize) -> usize {
        probe_cap + MAX_GROUP_WIDTH - 1
    }

    #[allow(dead_code)]
    fn memory_layout<E: Source>(
        capacity: usize,
        control_count: usize,
    ) -> Result<(Layout, usize), E> {
        let buckets_layout = Layout::array::<T>(capacity).into_error()?;
        let control_layout = Layout::array::<u8>(control_count).into_error()?;
        buckets_layout.extend(control_layout).into_error()
    }

    /// Serializes an iterator of items as a hash table.
    pub fn serialize_from_iter<I, U, H, S>(
        items: I,
        hashes: H,
        load_factor: (usize, usize),
        serializer: &mut S,
    ) -> Result<HashTableResolver, S::Error>
    where
        I: Clone + ExactSizeIterator,
        I::Item: Borrow<U>,
        U: Serialize<S, Archived = T>,
        H: ExactSizeIterator<Item = u64>,
        S: Fallible + Writer + Allocator + ?Sized,
        S::Error: Source,
    {
        #[derive(Debug)]
        struct InvalidLoadFactor {
            numerator: usize,
            denominator: usize,
        }

        impl fmt::Display for InvalidLoadFactor {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "invalid load factor {} / {}, load factor must be a \
                     fraction in the range (0, 1]",
                    self.numerator, self.denominator
                )
            }
        }

        impl Error for InvalidLoadFactor {}

        if load_factor.0 == 0
            || load_factor.1 == 0
            || load_factor.0 > load_factor.1
        {
            fail!(InvalidLoadFactor {
                numerator: load_factor.0,
                denominator: load_factor.1,
            });
        }

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

        let capacity = Self::capacity_from_len(len, load_factor);
        let probe_cap = Self::probe_cap(capacity);
        let control_count = Self::control_count(probe_cap);

        // Determine hash locations for all items
        SerVec::with_capacity(
            serializer,
            capacity,
            |ordered_items, serializer| {
                for _ in 0..capacity {
                    unsafe {
                        ordered_items.push_unchecked(None);
                    }
                }

                SerVec::<u8>::with_capacity(
                    serializer,
                    control_count,
                    |control_bytes, serializer| {
                        // Initialize all control bytes to EMPTY (0xFF)
                        unsafe {
                            control_bytes
                                .as_mut_ptr()
                                .write_bytes(0xff, control_bytes.capacity());
                            control_bytes.set_len(control_bytes.capacity());
                        }

                        let bucket_mask = Self::bucket_mask(control_count);

                        for (item, hash) in items.zip(hashes) {
                            let h2_hash = h2(hash);
                            let mut probe_seq = Self::probe_seq(hash, capacity);

                            'insert: loop {
                                for i in 0..MAX_GROUP_WIDTH / Group::WIDTH {
                                    let pos = probe_seq.pos + i * Group::WIDTH;
                                    let group = unsafe {
                                        Group::read(
                                            control_bytes.as_ptr().add(pos),
                                        )
                                    };

                                    if let Some(bit) =
                                        group.match_empty().lowest_set_bit()
                                    {
                                        let index = (pos + bit) % capacity;

                                        // Update control byte
                                        control_bytes[index] = h2_hash;
                                        // If it's near the beginning of the
                                        // control bytes,
                                        // update the wraparound control byte
                                        if index < (control_count - capacity) {
                                            control_bytes[capacity + index] =
                                                h2_hash;
                                        }

                                        ordered_items[index] = Some(item);
                                        break 'insert;
                                    }
                                }

                                loop {
                                    probe_seq.move_next(bucket_mask);
                                    if probe_seq.pos < probe_cap {
                                        break;
                                    }
                                }
                            }
                        }

                        let mut zeros = MaybeUninit::<T>::uninit();
                        unsafe {
                            zeros.as_mut_ptr().write_bytes(0, 1);
                        }
                        let zeros = unsafe {
                            from_raw_parts(
                                zeros.as_ptr().cast::<u8>(),
                                size_of::<T>(),
                            )
                        };
                        SerVec::with_capacity(
                            serializer,
                            len,
                            |resolvers, serializer| {
                                for item in ordered_items
                                    .iter()
                                    .filter_map(|x| x.as_ref())
                                {
                                    resolvers.push(
                                        item.borrow().serialize(serializer)?,
                                    );
                                }

                                serializer.align_for::<T>()?;

                                let mut resolvers = resolvers.drain().rev();
                                for item in ordered_items.iter().rev() {
                                    if let Some(item) = item {
                                        unsafe {
                                            serializer.resolve_aligned(
                                                item.borrow(),
                                                resolvers.next().unwrap(),
                                            )?;
                                        }
                                    } else {
                                        serializer.write(zeros)?;
                                    }
                                }

                                let pos = serializer.pos();
                                serializer.write(control_bytes)?;

                                Ok(HashTableResolver {
                                    pos: pos as FixedUsize,
                                })
                            },
                        )?
                    },
                )?
            },
        )?
    }

    /// Resolves an archived hash table from a given length and parameters.
    pub fn resolve_from_len(
        len: usize,
        load_factor: (usize, usize),
        resolver: HashTableResolver,
        out: Place<Self>,
    ) {
        munge!(let Self { ptr, len: out_len, cap, _phantom: _ } = out);

        if len == 0 {
            RawRelPtr::emplace_invalid(ptr);
        } else {
            RawRelPtr::emplace(resolver.pos as usize, ptr);
        }

        len.resolve((), out_len);

        let capacity = Self::capacity_from_len(len, load_factor);
        capacity.resolve((), cap);

        // PhantomData doesn't need to be initialized
    }
}

/// The resolver for [`ArchivedHashTable`].
pub struct HashTableResolver {
    pos: FixedUsize,
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

impl<T> RawIter<T> {
    /// Returns a raw iterator which yields no elements.
    pub fn empty() -> Self {
        Self {
            controls: ControlIter::none(),
            entries: NonNull::dangling(),
            items_left: 0,
        }
    }
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
    fn len(&self) -> usize {
        self.items_left
    }
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::{error::Error, fmt};

    use bytecheck::{CheckBytes, Verify};
    use rancor::{fail, Fallible, Source};

    use super::ArchivedHashTable;
    use crate::{
        simd::Group,
        validation::{ArchiveContext, ArchiveContextExt as _},
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
                "hash table length must be strictly less than its capacity \
                 (length: {}, capacity: {})",
                self.len, self.cap,
            )
        }
    }

    impl Error for InvalidLength {}

    #[derive(Debug)]
    struct UnwrappedControlByte {
        index: usize,
    }

    impl fmt::Display for UnwrappedControlByte {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "unwrapped control byte at index {}", self.index,)
        }
    }

    impl Error for UnwrappedControlByte {}

    unsafe impl<C, T> Verify<C> for ArchivedHashTable<T>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        T: CheckBytes<C>,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let len = self.len();
            let cap = self.capacity();

            if len == 0 && cap == 0 {
                return Ok(());
            }

            if len >= cap {
                fail!(InvalidLength { len, cap });
            }

            // Check memory allocation
            let probe_cap = Self::probe_cap(cap);
            let control_count = Self::control_count(probe_cap);
            let (layout, control_offset) =
                Self::memory_layout(cap, control_count)?;
            let ptr = self
                .ptr
                .as_ptr_wrapping()
                .cast::<u8>()
                .wrapping_sub(control_offset);

            context.in_subtree_raw(ptr, layout, |context| {
                // Check each non-empty bucket

                let this = (self as *const Self).cast_mut();
                // SAFETY: We have checked that `self` is not empty.
                let mut controls = unsafe { Self::control_iter(this) };
                let mut base_index = 0;
                'outer: while base_index < cap {
                    while let Some(bit) = controls.next_full() {
                        let index = base_index + bit;
                        if index >= cap {
                            break 'outer;
                        }

                        unsafe {
                            T::check_bytes(
                                Self::bucket_raw(this, index).as_ptr(),
                                context,
                            )?;
                        }
                    }

                    controls.move_next();
                    base_index += Group::WIDTH;
                }

                // Verify that wrapped bytes are set correctly
                for i in cap..usize::min(2 * cap, control_count - cap) {
                    let byte = unsafe { *Self::control_raw(this, i) };
                    let wrapped = unsafe { *Self::control_raw(this, i % cap) };
                    if wrapped != byte {
                        fail!(UnwrappedControlByte { index: i })
                    }
                }

                Ok(())
            })
        }
    }
}
