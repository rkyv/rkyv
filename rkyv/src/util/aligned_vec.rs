#[cfg(not(feature = "std"))]
use ::alloc::{alloc, boxed::Box, vec::Vec};
use core::borrow::{Borrow, BorrowMut};
use core::{
    fmt,
    ops::{Deref, DerefMut, Index, IndexMut},
    ptr::NonNull,
    slice,
};
#[cfg(feature = "std")]
use std::{alloc, io};

/// A vector of bytes that aligns its memory to 16 bytes.
pub struct AlignedVec {
    ptr: NonNull<u8>,
    cap: usize,
    len: usize,
}

impl Drop for AlignedVec {
    #[inline]
    fn drop(&mut self) {
        if self.cap != 0 {
            unsafe {
                alloc::dealloc(self.ptr.as_ptr(), self.layout());
            }
        }
    }
}

impl AlignedVec {
    /// The alignment of the vector
    pub const ALIGNMENT: usize = 16;

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
        AlignedVec {
            ptr: NonNull::dangling(),
            cap: 0,
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
    /// assert_eq!(vec.capacity(), 10);
    ///
    /// // These are all done without reallocating...
    /// for i in 0..10 {
    ///     vec.push(i);
    /// }
    /// assert_eq!(vec.len(), 10);
    /// assert_eq!(vec.capacity(), 10);
    ///
    /// // ...but this may make the vector reallocate
    /// vec.push(11);
    /// assert_eq!(vec.len(), 11);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        if capacity == 0 {
            Self::new()
        } else {
            let ptr = unsafe {
                alloc::alloc(alloc::Layout::from_size_align_unchecked(
                    capacity,
                    Self::ALIGNMENT,
                ))
            };
            Self {
                ptr: NonNull::new(ptr).unwrap(),
                cap: capacity,
                len: 0,
            }
        }
    }

    #[inline]
    fn layout(&self) -> alloc::Layout {
        unsafe { alloc::Layout::from_size_align_unchecked(self.cap, Self::ALIGNMENT) }
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

    #[inline]
    fn change_capacity(&mut self, new_cap: usize) {
        if new_cap != self.cap {
            let new_ptr = unsafe { alloc::realloc(self.ptr.as_ptr(), self.layout(), new_cap) };
            self.ptr = NonNull::new(new_ptr).unwrap();
            self.cap = new_cap;
        }
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
    /// assert_eq!(vec.capacity(), 10);
    /// vec.shrink_to_fit();
    /// assert!(vec.capacity() >= 3);
    /// ```
    #[inline]
    pub fn shrink_to_fit(&mut self) {
        if self.len == 0 {
            self.clear()
        } else {
            self.change_capacity(self.len);
        }
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
        self.ptr.as_ptr()
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
        unsafe { slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
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
        self.ptr.as_ptr()
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
        unsafe { slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }

    /// Returns the number of elements the vector can hold without reallocating.
    ///
    /// # Examples
    /// ```
    /// use rkyv::AlignedVec;
    ///
    /// let vec = AlignedVec::with_capacity(10);
    /// assert_eq!(vec.capacity(), 10);
    /// ```
    #[inline]
    pub fn capacity(&self) -> usize {
        self.cap
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
    /// vec.reserve(10);
    /// assert!(vec.capacity() >= 11);
    /// ```
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        let new_cap = self.len + additional;
        if new_cap > self.cap {
            let new_cap = new_cap
                .checked_next_power_of_two()
                .expect("cannot reserve a larger AlignedVec");
            if self.cap == 0 {
                let new_ptr = unsafe {
                    alloc::alloc(alloc::Layout::from_size_align_unchecked(
                        new_cap,
                        Self::ALIGNMENT,
                    ))
                };
                self.ptr = NonNull::new(new_ptr).unwrap();
                self.cap = new_cap;
            } else {
                let new_ptr = unsafe { alloc::realloc(self.ptr.as_ptr(), self.layout(), new_cap) };
                self.ptr = NonNull::new(new_ptr).unwrap();
                self.cap = new_cap;
            }
        }
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
                    self.as_mut_ptr().add(self.len()),
                    other.len(),
                );
            }
            self.len += other.len();
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
        unsafe {
            self.reserve(1);
            self.as_mut_ptr().add(self.len).write(value);
            self.len += 1;
        }
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
        let new_cap = self
            .len
            .checked_add(additional)
            .and_then(|n| n.checked_next_power_of_two())
            .expect("reserve amount overflowed");
        self.change_capacity(new_cap);
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
    /// assert_eq!(vec.capacity(), 10);
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

impl Clone for AlignedVec {
    #[inline]
    fn clone(&self) -> Self {
        unsafe {
            let mut result = AlignedVec::with_capacity(self.len);
            result.len = self.len;
            core::ptr::copy_nonoverlapping(self.as_ptr(), result.as_mut_ptr(), self.len);
            result
        }
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
