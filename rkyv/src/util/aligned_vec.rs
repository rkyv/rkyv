#[cfg(not(feature = "std"))]
use ::alloc::{alloc, boxed::Box, vec::Vec};
use core::borrow::{Borrow, BorrowMut};
use core::{
    cmp, fmt, hash,
    ops::{Deref, DerefMut, Index, IndexMut},
    slice,
    slice::SliceIndex,
};
#[cfg(feature = "std")]
use std::io;

use crate::{AlignedBytes, Archive, Deserialize, Serialize};

const ALIGNMENT: usize = 16;

// TODO: replace when int_roundings feature is stabilized
const fn div_ceiling(dividend: usize, divisor: usize) -> usize {
    (dividend + divisor - 1) / divisor
}

/// A vector of bytes that aligns its memory to 16 bytes.
///
/// The alignment also applies to [`ArchivedAlignedVec`], which is useful for aligning opaque bytes inside of an archived data
/// type.
///
/// ```
/// # use rkyv::{AlignedVec, Archive, Serialize};
/// #
/// #[derive(Archive, Serialize)]
/// struct HasAlignedBytes {
///     pub bytes: AlignedVec,
/// }
///
/// let mut bytes = AlignedVec::new();
/// bytes.extend_from_slice(&[1, 2, 3]);
/// let ser_foo = rkyv::to_bytes::<_, 0>(&HasAlignedBytes { bytes }).unwrap();
/// let arch_foo = unsafe { rkyv::archived_root::<HasAlignedBytes>(&ser_foo) };
/// assert_eq!(arch_foo.bytes.as_slice(), &[1, 2, 3]);
/// assert_eq!(arch_foo.bytes.as_ptr().align_offset(16), 0);
/// ```
#[derive(Archive, Clone, Deserialize, Serialize)]
#[archive(crate = "crate")]
pub struct AlignedVec {
    bytes: Vec<AlignedBytes<ALIGNMENT>>,
    len: usize,
}

impl AlignedVec {
    /// The alignment of the vector
    pub const ALIGNMENT: usize = ALIGNMENT;

    /// Constructs a new, empty `AlignedVec`.
    ///
    /// The vector will not allocate until elements are pushed into it.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            len: 0,
        }
    }

    /// Constructs a new, empty `AlignedVec` with the specified capacity.
    ///
    /// The vector will be able to hold exactly `capacity` bytes without reallocating. If
    /// `capacity` is 0, the vector will not allocate.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::with_capacity(10);
    ///
    /// // The vector contains no items, even though it has capacity for more
    /// assert_eq!(vec.len(), 0);
    /// assert_eq!(vec.capacity(), 16);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert_eq!(vec.capacity(), 16);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(div_ceiling(capacity, Self::ALIGNMENT)),
            len: 0,
        }
    }

    /// Clears the vector, removing all values.
    ///
    /// Note that this method has no effect on the allocated capacity of the vector.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut v = AlignedVec::new();
    /// v.extend_from_slice(&[1, 2, 3, 4]);
    ///
    /// v.clear();
    ///
    /// assert!(v.is_empty());
    /// ```
    #[inline]
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// Shrinks the capacity of the vector as much as possible.
    ///
    /// It will drop down as close as possible to the length but the allocator may still inform the
    /// vector that there is space for a few more elements.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::with_capacity(10);
    /// vec.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(vec.capacity(), 16);
    /// vec.shrink_to_fit();
    /// assert!(vec.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        self.bytes.shrink_to_fit();
    }

    /// Returns an unsafe mutable pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this function returns, or else
    /// it will end up pointing to garbage. Modifying the vector may cause its buffer to be
    /// reallocated, which would also make any pointers to it invalid.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// // Allocate vecotr big enough for 4 bytes.
    /// let size = 4;
    /// let mut x = AlignedVec::with_capacity(size);
    /// let x_ptr = x.as_mut_ptr();
    ///
    /// // Initialize elements via raw pointer writes, then set length.
    /// unsafe {
    ///     for i in 0..size {
    ///         *x_ptr.add(i) = i as u8;
    ///     }
    ///     x.set_len(size);
    /// }
    /// assert_eq!(&*x, &[0, 1, 2, 3]);
    /// ```
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.bytes.as_mut_ptr() as *mut u8
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    /// assert_eq!(vec.as_mut_slice().len(), 5);
    /// for i in 0..5 {
    ///     assert_eq!(vec.as_mut_slice()[i], i as u8 + 1);
    ///     vec.as_mut_slice()[i] = i as u8;
    ///     assert_eq!(vec.as_mut_slice()[i], i as u8);
    /// }
    /// ```
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    /// Returns a raw pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this function returns, or else
    /// it will end up pointing to garbage. Modifying the vector may cause its buffer to be
    /// reallocated, which would also make any pointers to it invalid.
    ///
    /// The caller must also ensure that the memory the pointer (non-transitively) points to is
    /// never written to (except inside an `UnsafeCell`) using this pointer or any pointer derived
    /// from it. If you need to mutate the contents of the slice, use
    /// [`as_mut_ptr`](AlignedVec::as_mut_ptr).
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut x = AlignedVec::new();
    /// x.extend_from_slice(&[1, 2, 4]);
    /// let x_ptr = x.as_ptr();
    ///
    /// unsafe {
    ///     for i in 0..x.len() {
    ///         assert_eq!(*x_ptr.add(i), 1 << i);
    ///     }
    /// }
    /// ```
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr() as *const u8
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.extend_from_slice(&[1, 2, 3, 4, 5]);
    /// assert_eq!(vec.as_slice().len(), 5);
    /// for i in 0..5 {
    ///     assert_eq!(vec.as_slice()[i], i as u8 + 1);
    /// }
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    /// Returns the number of elements the vector can hold without reallocating.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let vec = AlignedVec::with_capacity(10);
    /// assert_eq!(vec.capacity(), 16);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.bytes.capacity() * Self::ALIGNMENT
    }

    /// Reserves capacity for at least `additional` more bytes to be inserted into the given
    /// `AlignedVec`. The collection may reserve more space to avoid frequent reallocations. After
    /// calling `reserve`, capacity will be greater than or equal to `self.len() + additional`. Does
    /// nothing if capacity is already sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `usize::MAX` bytes.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.push(1);
    /// vec.reserve(16);
    /// assert!(vec.capacity() >= 17);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let required_cap = self.len + additional;
        if self.capacity() >= required_cap {
            return;
        }
        let additional_aligned = div_ceiling(required_cap - self.capacity(), Self::ALIGNMENT);
        self.bytes.reserve(additional_aligned)
    }

    /// Returns `true` if the vector contains no elements.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut v = Vec::new();
    /// assert!(v.is_empty());
    ///
    /// v.push(1);
    /// assert!(!v.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the vector, also referred to as its 'length'.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut a = AlignedVec::new();
    /// a.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(a.len(), 3);
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Copies and appends all bytes in a slice to the `AlignedVec`.
    ///
    /// The elements of the slice are appended in-order.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.push(1);
    /// vec.extend_from_slice(&[2, 3, 4]);
    /// assert_eq!(vec.as_slice(), &[1, 2, 3, 4]);
    /// ```
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[u8]) {
        if !other.is_empty() {
            self.reserve(other.len());
            unsafe {
                core::ptr::copy_nonoverlapping(
                    other.as_ptr(),
                    self.as_mut_ptr().add(self.len),
                    other.len(),
                );
            }
            unsafe { self.set_len(self.len + other.len()) };
        }
    }

    /// Removes the last element from a vector and returns it, or `None` if it is empty.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.extend_from_slice(&[1, 2, 3]);
    /// assert_eq!(vec.pop(), Some(3));
    /// assert_eq!(vec.as_slice(), &[1, 2]);
    /// ```
    #[inline]
    pub fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            None
        } else {
            let result = self[self.len - 1];
            self.len -= 1;
            Some(result)
        }
    }

    /// Appends an element to the back of a collection.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity exceeds `usize::MAX` bytes.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.extend_from_slice(&[1, 2]);
    /// vec.push(3);
    /// assert_eq!(vec.as_slice(), &[1, 2, 3]);
    /// ```
    #[inline]
    pub fn push(&mut self, value: u8) {
        self.reserve(1);
        unsafe {
            self.as_mut_ptr().add(self.len).write(value);
        }
        self.len += 1;
    }

    /// Reserves the minimum capacity for exactly `additional` more elements to be inserted in the
    /// given `AlignedVec`. After calling `reserve_exact`, capacity will be greater than or equal
    /// to `self.len() + additional`. Does nothing if the capacity is already sufficient.
    ///
    /// Note that the allocator may give the collection more space than it requests. Therefore,
    /// capacity can not be relied upon to be precisely minimal. Prefer reserve if future insertions
    /// are expected.
    ///
    /// # Panics
    ///
    /// Panics if the new capacity overflows `usize`.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::new();
    /// vec.push(1);
    /// vec.reserve_exact(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        let required_cap = self.len + additional;
        if self.capacity() >= required_cap {
            return;
        }
        let additional_aligned = div_ceiling(required_cap - self.capacity(), Self::ALIGNMENT);
        self.bytes.reserve_exact(additional_aligned)
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal invariants of the type.
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity()`](AlignedVec::capacity)
    /// - The elements at `old_len..new_len` must be initialized
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::with_capacity(3);
    /// vec.extend_from_slice(&[1, 2, 3]);
    ///
    /// // SAFETY:
    /// // 1. `old_len..0` is empty to no elements need to be initialized.
    /// // 2. `0 <= capacity` always holds whatever capacity is.
    /// unsafe {
    ///     vec.set_len(0);
    /// }
    /// ```
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());
        self.len = new_len;
        // It's important to set_len here as well so that reserve and reserve_exact work as expected.
        self.bytes.set_len(div_ceiling(new_len, Self::ALIGNMENT));
    }

    /// Converts the vector into `Box<[u8]>`.
    ///
    /// This method reallocates and copies the underlying bytes. Any excess capacity is dropped.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut v = AlignedVec::new();
    /// v.extend_from_slice(&[1, 2, 3]);
    ///
    /// let slice = v.into_boxed_slice();
    /// ```
    ///
    /// Any excess capacity is removed:
    ///
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut vec = AlignedVec::with_capacity(10);
    /// vec.extend_from_slice(&[1, 2, 3]);
    ///
    /// assert_eq!(vec.capacity(), 16);
    /// let slice = vec.into_boxed_slice();
    /// assert_eq!(slice.len(), 3);
    /// ```
    #[inline]
    pub fn into_boxed_slice(self) -> Box<[u8]> {
        self.into_vec().into_boxed_slice()
    }

    /// Converts the vector into `Vec<u8>`.
    ///
    /// This method reallocates and copies the underlying bytes. Any excess capacity is dropped.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let mut v = AlignedVec::new();
    /// v.extend_from_slice(&[1, 2, 3]);
    ///
    /// let vec = v.into_vec();
    /// assert_eq!(vec.len(), 3);
    /// assert_eq!(vec.as_slice(), &[1, 2, 3]);
    /// ```
    #[inline]
    pub fn into_vec(self) -> Vec<u8> {
        Vec::from(self.as_ref())
    }
}

impl From<AlignedVec> for Vec<u8> {
    #[inline]
    fn from(aligned: AlignedVec) -> Self {
        aligned.to_vec()
    }
}

impl AsMut<[u8]> for AlignedVec {
    #[inline]
    fn as_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl AsRef<[u8]> for AlignedVec {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for AlignedVec {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl BorrowMut<[u8]> for AlignedVec {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

impl fmt::Debug for AlignedVec {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl Default for AlignedVec {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for AlignedVec {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for AlignedVec {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<I: slice::SliceIndex<[u8]>> Index<I> for AlignedVec {
    type Output = <I as slice::SliceIndex<[u8]>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<I: slice::SliceIndex<[u8]>> IndexMut<I> for AlignedVec {
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

#[cfg(feature = "std")]
impl io::Write for AlignedVec {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.extend_from_slice(buf);
        Ok(buf.len())
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        let len = bufs.iter().map(|b| b.len()).sum();
        self.reserve(len);
        for buf in bufs {
            self.extend_from_slice(buf);
        }
        Ok(len)
    }

    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.extend_from_slice(buf);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// SAFETY: AlignedVec is safe to send to another thread
unsafe impl Send for AlignedVec {}

// SAFETY: AlignedVec is safe to share between threads
unsafe impl Sync for AlignedVec {}

impl Unpin for AlignedVec {}

impl ArchivedAlignedVec {
    /// Returns the number of elements in the archived vec.
    #[inline]
    pub fn len(&self) -> usize {
        from_archived!(self.len) as usize
    }

    /// Returns whether the archived vec is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Gets the elements of the archived vec as a slice.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr() as *const u8
    }

    /// Gets the elements of the archived vec as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }
}

impl AsRef<[u8]> for ArchivedAlignedVec {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Borrow<[u8]> for ArchivedAlignedVec {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self.as_slice()
    }
}

impl fmt::Debug for ArchivedAlignedVec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.as_slice()).finish()
    }
}

impl Deref for ArchivedAlignedVec {
    type Target = [u8];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl Eq for ArchivedAlignedVec {}

impl hash::Hash for ArchivedAlignedVec {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state)
    }
}

impl<I: SliceIndex<[u8]>> Index<I> for ArchivedAlignedVec {
    type Output = <[u8] as Index<I>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        self.as_slice().index(index)
    }
}

impl Ord for ArchivedAlignedVec {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}

impl PartialEq<ArchivedAlignedVec> for ArchivedAlignedVec {
    #[inline]
    fn eq(&self, other: &ArchivedAlignedVec) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<const N: usize> PartialEq<[u8; N]> for ArchivedAlignedVec {
    #[inline]
    fn eq(&self, other: &[u8; N]) -> bool {
        self.as_slice().eq(&other[..])
    }
}

impl<const N: usize> PartialEq<ArchivedAlignedVec> for [u8; N] {
    #[inline]
    fn eq(&self, other: &ArchivedAlignedVec) -> bool {
        other.eq(self)
    }
}

impl PartialEq<[u8]> for ArchivedAlignedVec {
    #[inline]
    fn eq(&self, other: &[u8]) -> bool {
        self.as_slice().eq(other)
    }
}

impl PartialEq<ArchivedAlignedVec> for [u8] {
    #[inline]
    fn eq(&self, other: &ArchivedAlignedVec) -> bool {
        self.eq(other.as_slice())
    }
}

impl PartialOrd<ArchivedAlignedVec> for ArchivedAlignedVec {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedAlignedVec) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl PartialOrd<[u8]> for ArchivedAlignedVec {
    #[inline]
    fn partial_cmp(&self, other: &[u8]) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedAlignedVec> for [u8] {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedAlignedVec) -> Option<cmp::Ordering> {
        self.partial_cmp(other.as_slice())
    }
}
