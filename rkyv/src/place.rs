//! An initialized, writeable location in memory.

use core::{mem::size_of, ptr::NonNull};

use munge::{Borrow, Destructure, Restructure};

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
    #[inline]
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
    #[inline]
    pub unsafe fn from_field_unchecked<U: ?Sized>(
        parent: Place<U>,
        ptr: *mut T,
    ) -> Self {
        let offset = ptr as *mut () as usize - parent.ptr() as *mut () as usize;
        Self::new_unchecked(parent.pos() + offset, ptr)
    }

    /// Returns the position of the place.
    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Returns the pointer associated with this place.
    ///
    /// # Safety
    ///
    /// Uninitialized bytes must not be written to the returned pointer.
    #[inline]
    pub unsafe fn ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Writes the provided value to this place.
    ///
    /// # Safety
    ///
    /// `value` must not have any uninitialized bytes (e.g. padding).
    #[inline]
    pub unsafe fn write_unchecked(&self, value: T)
    where
        T: Sized,
    {
        unsafe {
            self.ptr().write(value);
        }
    }

    /// Writes the provided value to this place.
    #[inline]
    pub fn write(&self, value: T)
    where
        T: Initialized + Sized,
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
    #[inline]
    pub unsafe fn cast_unchecked<U>(&self) -> Place<U>
    where
        T: Sized,
    {
        Place {
            pos: self.pos,
            ptr: self.ptr.cast(),
        }
    }
}

impl<T> Place<[T]> {
    /// Gets a `Place` to the `i`-th element of the slice.
    ///
    /// # Safety
    ///
    /// `i` must be in-bounds for the slice pointed to by this place.
    #[inline]
    pub unsafe fn index(&self, i: usize) -> Place<T> {
        Place::new_unchecked(self.pos() + i * size_of::<T>(), unsafe {
            self.ptr().cast::<T>().add(i)
        })
    }
}

impl<T, const N: usize> Place<[T; N]> {
    /// Gets a `Place` to the `i`-th element of the array.
    ///
    /// # Safety
    ///
    /// `i` must be in-bounds for the array pointed to by this place.
    #[inline]
    pub unsafe fn index(&self, i: usize) -> Place<T> {
        Place::new_unchecked(self.pos() + i * size_of::<T>(), unsafe {
            self.ptr().cast::<T>().add(i)
        })
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
        Place::from_field_unchecked(*self, ptr)
    }
}

/// A marker trait which indicates that all of the bytes of a type are
/// fully-initialized.
///
/// As a result, `Initialized` types may not contain padding.
///
/// # Safety
///
/// The bytes of types implementing `Initialized` must always be
/// fully-initialized. Among other things, this means they may not contain
/// padding.
pub unsafe trait Initialized {}

// SAFETY: An array of values which are all fully-initialized is also
// fully-initalized.
unsafe impl<T: Initialized, const N: usize> Initialized for [T; N] {}
