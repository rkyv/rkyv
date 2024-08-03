use core::{
    borrow::{Borrow, BorrowMut},
    fmt,
    marker::PhantomData,
    mem::MaybeUninit,
    ops,
    ptr::{self, NonNull},
    slice::{self, from_raw_parts_mut},
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
    pub fn new() -> Self {
        Self {
            elements: unsafe { MaybeUninit::uninit().assume_init() },
            len: 0,
        }
    }

    /// Clears the vector, removing all values.
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
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self.elements.as_mut_ptr().cast()
    }

    /// Extracts a mutable slice of the entire vector.
    ///
    /// Equivalent to `&mut s[..]`.
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
    pub fn as_ptr(&self) -> *const T {
        self.elements.as_ptr().cast()
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to `&s[..]`.
    pub fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len) }
    }

    /// Returns the number of elements the vector can hole without reallocating.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Ensures that there is capacity for at least `additional` more elements
    /// to be inserted into the `ScratchVec`.
    ///
    /// # Panics
    ///
    /// Panics if the required capacity exceeds the available capacity.
    pub fn reserve(&mut self, additional: usize) {
        if N - self.len < additional {
            Self::out_of_space();
        }
    }

    #[cold]
    fn out_of_space() -> ! {
        panic!(
            "reserve requested more capacity than the InlineVec has available"
        );
    }

    /// Returns `true` if the vector contains no elements.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the number of elements in the vector, also referred to as its
    /// `length`.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Copies and appends all elements in a slice to the `ScratchVec`.
    ///
    /// The elements of the slice are appended in-order.
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

    /// Appends an element to the back of a collection without performing bounds
    /// checking.
    ///
    /// # Safety
    ///
    /// The vector must have enough space reserved for the pushed element.
    pub unsafe fn push_unchecked(&mut self, value: T) {
        unsafe {
            self.as_mut_ptr().add(self.len).write(value);
            self.len += 1;
        }
    }

    /// Appends an element to the back of a collection.
    pub fn push(&mut self, value: T) {
        if self.len == N {
            Self::out_of_space()
        } else {
            unsafe {
                self.push_unchecked(value);
            }
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
    pub unsafe fn set_len(&mut self, new_len: usize) {
        debug_assert!(new_len <= self.capacity());

        self.len = new_len;
    }

    /// Creates a draining iterator that removes all of the elements from the
    /// vector.
    pub fn drain(&mut self) -> Drain<'_, T, N> {
        let remaining = self.len();
        unsafe {
            self.set_len(0);
        }

        Drain {
            current: unsafe { NonNull::new_unchecked(self.as_mut_ptr()) },
            remaining,
            _phantom: PhantomData,
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
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> AsRef<[T]> for InlineVec<T, N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> Borrow<[T]> for InlineVec<T, N> {
    fn borrow(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> BorrowMut<[T]> for InlineVec<T, N> {
    fn borrow_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for InlineVec<T, N> {
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

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> ops::DerefMut for InlineVec<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, I: slice::SliceIndex<[T]>, const N: usize> ops::Index<I>
    for InlineVec<T, N>
{
    type Output = <I as slice::SliceIndex<[T]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T, I: slice::SliceIndex<[T]>, const N: usize> ops::IndexMut<I>
    for InlineVec<T, N>
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

/// A draining iterator for `InlineVec<T>`.
///
/// This `struct` is created by [`InlineVec::drain`]. See its documentation for
/// more.
pub struct Drain<'a, T: 'a, const N: usize> {
    current: NonNull<T>,
    remaining: usize,
    _phantom: PhantomData<&'a mut InlineVec<T, N>>,
}

impl<T: fmt::Debug, const N: usize> fmt::Debug for Drain<'_, T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Drain").field(&self.as_slice()).finish()
    }
}

impl<T, const N: usize> Drain<'_, T, N> {
    /// Returns the remaining items of this iterator as a slice.
    pub fn as_slice(&self) -> &[T] {
        unsafe { from_raw_parts_mut(self.current.as_ptr(), self.remaining) }
    }
}

impl<T, const N: usize> AsRef<[T]> for Drain<'_, T, N> {
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T, const N: usize> Iterator for Drain<'_, T, N> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        if self.remaining > 0 {
            self.remaining -= 1;
            let result = unsafe { self.current.as_ptr().read() };
            self.current =
                unsafe { NonNull::new_unchecked(self.current.as_ptr().add(1)) };
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<T, const N: usize> DoubleEndedIterator for Drain<'_, T, N> {
    fn next_back(&mut self) -> Option<T> {
        if self.remaining > 0 {
            self.remaining -= 1;
            unsafe { Some(self.current.as_ptr().add(self.remaining).read()) }
        } else {
            None
        }
    }
}

impl<T, const N: usize> Drop for Drain<'_, T, N> {
    fn drop(&mut self) {
        for i in 0..self.remaining {
            unsafe {
                self.current.as_ptr().add(i).drop_in_place();
            }
        }
    }
}

impl<T, const N: usize> ExactSizeIterator for Drain<'_, T, N> {}

impl<T, const N: usize> core::iter::FusedIterator for Drain<'_, T, N> {}

#[cfg(test)]
mod tests {
    use crate::util::InlineVec;

    #[test]
    fn drain() {
        let mut vec = InlineVec::<_, 8>::new();

        for i in 0..100 {
            vec.push(i);
            if vec.len() == vec.capacity() {
                for j in vec.drain() {
                    let _ = j;
                }
            }
        }
    }
}
