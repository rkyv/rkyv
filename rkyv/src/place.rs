//! An initialized, writeable location in memory.

use core::{mem::size_of, ptr::NonNull};

use munge::{Borrow, Destructure, Restructure};

use crate::traits::{LayoutRaw, NoUndef};

/// A place to write a `T` paired with its position in the output buffer.
pub struct Place<T: ?Sized> {
    pos: usize,
    ptr: NonNull<T>,
}

impl<T: ?Sized> Clone for Place<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Place<T> {}

impl<T: ?Sized> Place<T> {
    /// Creates a new `Place` from an output pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be properly aligned, dereferenceable, and all of its bytes
    /// must be initialized.
    pub unsafe fn new_unchecked(pos: usize, ptr: *mut T) -> Self {
        unsafe {
            Self {
                pos,
                ptr: NonNull::new_unchecked(ptr),
            }
        }
    }

    /// Creates a new `Place` from a parent pointer and the field the place
    /// points to.
    ///
    /// # Safety
    ///
    /// - `ptr` must point to a field of `parent`
    /// - `ptr` must be properly aligned, dereferenceable, and all of its bytes
    ///   must be initialized
    pub unsafe fn from_field_unchecked<U: ?Sized>(
        parent: Place<U>,
        ptr: *mut T,
    ) -> Self {
        // SAFETY: We won't write anything to the parent pointer, so we
        // definitely won't write any uninitialized bytes.
        let parent_ptr = unsafe { parent.ptr() };
        let offset = ptr as *mut () as usize - parent_ptr as *mut () as usize;
        // SAFETY: The caller has guaranteed that `ptr` is properly aligned,
        // dereferenceable, and all of its bytes are initialized.
        unsafe { Self::new_unchecked(parent.pos() + offset, ptr) }
    }

    /// Returns the position of the place.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the pointer associated with this place.
    ///
    /// # Safety
    ///
    /// Uninitialized bytes must not be written to the returned pointer.
    pub unsafe fn ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Writes the provided value to this place.
    ///
    /// # Safety
    ///
    /// `value` must not have any uninitialized bytes (e.g. padding).
    pub unsafe fn write_unchecked(&self, value: T)
    where
        T: Sized,
    {
        unsafe {
            self.ptr().write(value);
        }
    }

    /// Writes the provided value to this place.
    pub fn write(&self, value: T)
    where
        T: NoUndef + Sized,
    {
        unsafe {
            self.write_unchecked(value);
        }
    }

    /// Returns this place casted to the given type.
    ///
    /// # Safety
    ///
    /// This place must point to a valid `U`.
    pub unsafe fn cast_unchecked<U>(&self) -> Place<U>
    where
        T: Sized,
    {
        Place {
            pos: self.pos,
            ptr: self.ptr.cast(),
        }
    }

    /// Returns a slice of the bytes this place points to.
    pub fn as_slice(&self) -> &[u8]
    where
        T: LayoutRaw,
    {
        let ptr = self.ptr.as_ptr();
        let len = T::layout_raw(ptr_meta::metadata(ptr)).unwrap().size();
        // SAFETY: The pointers of places are always properly aligned and
        // dereferenceable. All of the bytes this place points to are guaranteed
        // to be initialized at all times.
        unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) }
    }
}

impl<T> Place<[T]> {
    /// Gets a `Place` to the `i`-th element of the slice.
    ///
    /// # Safety
    ///
    /// `i` must be in-bounds for the slice pointed to by this place.
    pub unsafe fn index(&self, i: usize) -> Place<T> {
        // SAFETY: The caller has guaranteed that `i` is in-bounds for the slice
        // pointed to by this place.
        let ptr = unsafe { self.ptr().cast::<T>().add(i) };
        // SAFETY: `ptr` is an element of `self`, and so is also properly
        // aligned, dereferenceable, and all of its bytes are initialized.
        unsafe { Place::new_unchecked(self.pos() + i * size_of::<T>(), ptr) }
    }
}

impl<T, const N: usize> Place<[T; N]> {
    /// Gets a `Place` to the `i`-th element of the array.
    ///
    /// # Safety
    ///
    /// `i` must be in-bounds for the array pointed to by this place.
    pub unsafe fn index(&self, i: usize) -> Place<T> {
        // SAFETY: The caller has guaranteed that `i` is in-bounds for the array
        // pointed to by this place.
        let ptr = unsafe { self.ptr().cast::<T>().add(i) };
        // SAFETY: `ptr` is an element of `self`, and so is also properly
        // aligned, dereferenceable, and all of its bytes are initialized.
        unsafe { Place::new_unchecked(self.pos() + i * size_of::<T>(), ptr) }
    }
}

unsafe impl<T: ?Sized> Destructure for Place<T> {
    type Underlying = T;
    type Destructuring = Borrow;

    fn underlying(&mut self) -> *mut Self::Underlying {
        self.ptr.as_ptr()
    }
}

unsafe impl<T: ?Sized, U: ?Sized> Restructure<U> for Place<T> {
    type Restructured = Place<U>;

    unsafe fn restructure(&self, ptr: *mut U) -> Self::Restructured {
        // SAFETY: `ptr` is a pointer to a subfield of the underlying pointer,
        // and so is also properly aligned, dereferenceable, and all of its
        // bytes are initialized.
        unsafe { Place::from_field_unchecked(*self, ptr) }
    }
}
