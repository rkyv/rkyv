use core::{
    borrow::{Borrow, BorrowMut},
    fmt,
    mem::MaybeUninit,
    ops,
    ptr::{self, NonNull},
    slice,
};

/// A vector that uses inline-allocated memory.
pub struct InlineVec<T, const N: usize> {
    elements: [MaybeUninit<T>; N],
    len: usize,
}

impl<T, const N: usize> Drop for InlineVec<T, N> {
    fn drop(&mut self) {
        self.clear()
    }
}

// SAFETY: InlineVec is safe to send to another thread is T is safe to send to
// another thread
unsafe impl<T: Send, const N: usize> Send for InlineVec<T, N> {}

// SAFETY: InlineVec is safe to share between threads if T is safe to share
// between threads
unsafe impl<T: Sync, const N: usize> Sync for InlineVec<T, N> {}

impl<T, const N: usize> InlineVec<T, N> {
    /// Constructs a new, empty `InlineVec`.
    ///
    /// The vector will be able to hold exactly `N` elements.
    #[inline]
    pub fn new() -> Self {
        Self {
            elements: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    /// Clears the vector, removing all values.
    #[inline]
    pub fn clear(&mut self) {
        for i in 0..self.len {
            unsafe {
                self.elements[i].as_mut_ptr().drop_in_place();
            }
        }
        self.len = 0;
    }

    /// Returns an unsafe mutable pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// function returns, or else it will end up pointing to garbage.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.elements.as_mut_ptr().cast()
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len) }
    }

    /// Returns a raw pointer to the vector's buffer.
    ///
    /// The caller must ensure that the vector outlives the pointer this
    /// functions returns, or else it will end up pointing to garbage.
    ///
    /// The caller must also ensure that the memory the pointer
    /// (non-transitively) points to is never written to (except inside an
    /// `UnsafeCell`) using this pointer or any pointer derived from it. If
    /// you need to mutate the contents of the slice, use
    /// [`as_mut_ptr`](Self::as_mut_ptr).
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self.elements.as_ptr().cast()
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    /// Returns the number of elements the vector can hole without reallocating.
    #[inline]
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Ensures that there is capacity for at least `additional` more elements
    /// to be inserted into the `ScratchVec`.
    ///
    /// # Panics
    ///
    /// Panics if the required capacity exceeds the available capacity.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        if self.len + additional > N {
            panic!(
                "reserve requested more capacity than the scratch vec has \
                 available"
            );
        }
    }

    /// Returns `true` if the vector contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the vector, also referred to as its
    /// `length`.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Copies and appends all elements in a slice to the `ScratchVec`.
    ///
    /// The elements of the slice are appended in-order.
    #[inline]
    pub fn extend_from_slice(&mut self, other: &[T]) {
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

    /// Removes the last element from a vector and returns it, or `None` if it
    /// is empty.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            unsafe {
                self.len -= 1;
                Some(self.as_ptr().add(self.len()).read())
            }
        }
    }

    /// Appends an element to the back of a collection.
    #[inline]
    pub fn push(&mut self, value: T) {
        unsafe {
            self.reserve(1);
            self.as_mut_ptr().add(self.len).write(value);
            self.len += 1;
        }
    }

    /// Reserves the minimum capacity for exactly `additional` more elements to
    /// be inserted in the given `AlignedVec`. After calling
    /// `reserve_exact`, capacity will be greater than or equal
    /// to `self.len() + additional`. Does nothing if the capacity is already
    /// sufficient.
    ///
    /// # Panics
    ///
    /// Panics if the required capacity exceeds the available capacity.
    #[inline]
    pub fn reserve_exact(&mut self, additional: usize) {
        self.reserve(additional);
    }

    /// Forces the length of the vector to `new_len`.
    ///
    /// This is a low-level operation that maintains none of the normal
    /// invariants of the type.
    ///
    /// # Safety
    ///
    /// - `new_len` must be less than or equal to [`capacity()`](Self::capacity)
    /// - The elements at `old_len..new_len` must be initialized
    #[inline]
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());

        self.len = new_len;
    }

    // This is taken from `slice::range`, which is not yet stable.
    #[inline]
    fn drain_range<R>(
        range: R,
        bounds: ops::RangeTo<usize>,
    ) -> ops::Range<usize>
    where
        R: ops::RangeBounds<usize>,
    {
        let len = bounds.end;

        let start: ops::Bound<&usize> = range.start_bound();
        let start = match start {
            ops::Bound::Included(&start) => start,
            ops::Bound::Excluded(start) => {
                start.checked_add(1).unwrap_or_else(|| {
                    panic!("attempted to index slice from after maximum usize")
                })
            }
            ops::Bound::Unbounded => 0,
        };

        let end: ops::Bound<&usize> = range.end_bound();
        let end = match end {
            ops::Bound::Included(end) => {
                end.checked_add(1).unwrap_or_else(|| {
                    panic!("attempted to index slice up to maximum usize")
                })
            }
            ops::Bound::Excluded(&end) => end,
            ops::Bound::Unbounded => len,
        };

        if start > end {
            panic!("slice index starts at {} but ends at {}", start, end);
        }
        if end > len {
            panic!(
                "range start index {} out of range for slice of length {}",
                end, len
            );
        }

        ops::Range { start, end }
    }

    /// Creates a draining iterator that removes the specified range in the
    /// vector and yields the removed items.
    ///
    /// When the iterator **is** dropped, all elements in the range are removed
    /// from the vector, even if the iterator was not fully consumed. If the
    /// iterator **is not** dropped (with `mem::forget` for example), it is
    /// unspecified how many elements are removed.
    ///
    /// # Panics
    ///
    /// Panics if the starting point is greater than the end point or if the end
    /// point is greater than the length of the vector.
    #[inline]
    pub fn drain<R: ops::RangeBounds<usize>>(
        &mut self,
        range: R,
    ) -> Drain<'_, T, N> {
        let len = self.len();
        let ops::Range { start, end } = Self::drain_range(range, ..len);

        unsafe {
            self.set_len(start);
            let range_slice = slice::from_raw_parts_mut(
                self.as_mut_ptr().add(start),
                end - start,
            );
            Drain {
                tail_start: end,
                tail_len: len - end,
                iter: range_slice.iter(),
                vec: NonNull::from(self),
            }
        }
    }
}

impl<T, const N: usize> InlineVec<MaybeUninit<T>, N> {
    /// Assuming that all the elements are initialized, removes the
    /// `MaybeUninit` wrapper from the vector.
    ///
    /// # Safety
    ///
    /// It is up to the caller to guarantee that the `MaybeUninit<T>` elements
    /// really are in an initialized state. Calling this when the content is
    /// not yet fully initialized causes undefined behavior.
    #[inline]
    pub fn assume_init(self) -> InlineVec<T, N> {
        let mut elements = unsafe {
            MaybeUninit::<[MaybeUninit<T>; N]>::uninit().assume_init()
        };
        unsafe {
            ptr::copy_nonoverlapping(
                self.elements.as_ptr().cast(),
                elements.as_mut_ptr(),
                N,
            );
        }
        InlineVec {
            elements,
            len: self.len,
        }
    }
}

impl<T, const N: usize> AsMut<[T]> for InlineVec<T, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> AsRef<[T]> for InlineVec<T, N> {
    #[inline]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> Borrow<[T]> for InlineVec<T, N> {
    #[inline]
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> BorrowMut<[T]> for InlineVec<T, N> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for InlineVec<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_slice().fmt(f)
    }
}

impl<T, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize> ops::Deref for InlineVec<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> ops::DerefMut for InlineVec<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, I: slice::SliceIndex<[T]>, const N: usize> ops::Index<I>
    for InlineVec<T, N>
{
    type Output = <I as slice::SliceIndex<[T]>>::Output;

    #[inline]
    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T, I: slice::SliceIndex<[T]>, const N: usize> ops::IndexMut<I>
    for InlineVec<T, N>
{
    #[inline]
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

/// A draining iterator for `ScratchVec<T>`.
///
/// This `struct` is created by [`ScratchVec::drain`]. See its documentation for
/// more.
pub struct Drain<'a, T: 'a, const N: usize> {
    tail_start: usize,
    tail_len: usize,
    iter: slice::Iter<'a, T>,
    vec: NonNull<InlineVec<T, N>>,
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Drain<'_, T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.iter.as_slice()).finish()
    }
}

impl<T, const N: usize> Drain<'_, T, N> {
    /// Returns the remaining items of this iterator as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.iter.as_slice()
    }
}

impl<T, const N: usize> AsRef<[T]> for Drain<'_, T, N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> Iterator for Drain<'_, T, N> {
    type Item = T;

    #[inline]
    fn next(&mut self) -> Option<T> {
        self.iter
            .next()
            .map(|elt| unsafe { core::ptr::read(elt as *const _) })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T, const N: usize> DoubleEndedIterator for Drain<'_, T, N> {
    #[inline]
    fn next_back(&mut self) -> Option<T> {
        self.iter
            .next_back()
            .map(|elt| unsafe { core::ptr::read(elt as *const _) })
    }
}

impl<T, const N: usize> Drop for Drain<'_, T, N> {
    fn drop(&mut self) {
        /// Continues dropping the remaining elements in the `Drain`, then moves
        /// back the un-`Drain`ed elements to restore the original
        /// `Vec`.
        struct DropGuard<'r, 'a, T, const N: usize>(&'r mut Drain<'a, T, N>);

        impl<'r, 'a, T, const N: usize> Drop for DropGuard<'r, 'a, T, N> {
            fn drop(&mut self) {
                // Continue the same loop we have below. If the loop already
                // finished, this does nothing.
                self.0.for_each(drop);

                if self.0.tail_len > 0 {
                    unsafe {
                        let source_vec = self.0.vec.as_mut();
                        // memmove back untouched tail, update to new length
                        let start = source_vec.len();
                        let tail = self.0.tail_start;
                        if tail != start {
                            let src = source_vec.as_ptr().add(tail);
                            let dst = source_vec.as_mut_ptr().add(start);
                            core::ptr::copy(src, dst, self.0.tail_len);
                        }
                        source_vec.set_len(start + self.0.tail_len);
                    }
                }
            }
        }

        // exhaust self first
        while let Some(item) = self.next() {
            let guard = DropGuard(self);
            drop(item);
            core::mem::forget(guard);
        }

        // Drop a `DropGuard` to move back the non-drained tail of `self`.
        DropGuard(self);
    }
}

impl<T, const N: usize> ExactSizeIterator for Drain<'_, T, N> {}

impl<T, const N: usize> core::iter::FusedIterator for Drain<'_, T, N> {}
