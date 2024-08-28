//! Mutable references to values which may not be moved or de-initialized.

use core::{
    ops::{Deref, DerefMut},
    slice::SliceIndex,
};

use munge::{Borrow, Destructure, Restructure};

use crate::traits::NoUndef;

/// A mutable reference which may not be moved or assigned.
///
/// A `Seal` restricts a mutable reference so that the referenced value cannot
/// be moved or assigned unless it is `Unpin` and `NoUndef`. These properties
/// allow the safe use of mutable archived values.
///
/// Unlike `Pin`, all fields of `Seal`ed values are also sealed. There is no
/// notion of "structural sealing" as there is structural pinning. This has the
/// upside that a `Seal` can be uniformly destructured with `munge`, which is
/// the recommended replacement for `Pin`'s `map_unchecked_mut` function. Also
/// unlike `Pin`, `Seal`ing a reference does not require upholding the invariant
/// that the sealed value is dropped before its backing memory is reused. This
/// means that creating a `Seal` from a mutable reference is completely safe to
/// do.
pub struct Seal<'a, T: ?Sized> {
    inner: &'a mut T,
}

impl<'a, T: ?Sized> Seal<'a, T> {
    /// Returns a new `Seal` wrapping the given reference.
    pub fn new(inner: &'a mut T) -> Self {
        Self { inner }
    }

    /// Returns the underlying reference for types that implement `NoUndef`
    /// and `Unpin`.
    pub fn unseal(self) -> &'a mut T
    where
        T: NoUndef + Unpin,
    {
        self.inner
    }

    /// Returns the underlying reference as shared for types that implement
    /// `Portable`.
    pub fn unseal_ref(self) -> &'a T {
        self.inner
    }

    /// Returns the underlying reference.
    ///
    /// # Safety
    ///
    /// The returned reference may not be moved unless `T` is `Unpin`.
    /// Uninitialized bytes may not be written through the `Seal`.
    pub unsafe fn unseal_unchecked(self) -> &'a mut T {
        self.inner
    }

    /// Mutably reborrows the `Seal`.
    pub fn as_mut(&mut self) -> Seal<'_, T> {
        Seal::new(self.inner)
    }
}

impl<T: ?Sized> AsRef<T> for Seal<'_, T> {
    fn as_ref(&self) -> &T {
        self.inner
    }
}

impl<T: ?Sized> Deref for Seal<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: NoUndef + Unpin + ?Sized> DerefMut for Seal<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut().unseal()
    }
}

unsafe impl<T: ?Sized> Destructure for Seal<'_, T> {
    type Underlying = T;
    type Destructuring = Borrow;

    fn underlying(&mut self) -> *mut Self::Underlying {
        self.inner
    }
}

unsafe impl<'a, T: ?Sized, U: 'a + ?Sized> Restructure<U> for Seal<'a, T> {
    type Restructured = Seal<'a, U>;

    unsafe fn restructure(&self, ptr: *mut U) -> Self::Restructured {
        // SAFETY: `ptr` is a pointer to a subfield of the underlying pointer,
        // and so is also properly aligned, and dereferenceable.
        Seal::new(unsafe { &mut *ptr })
    }
}

impl<'a, T> Seal<'a, [T]> {
    /// Indexes the `Seal`.
    ///
    /// # Panics
    ///
    /// May panic if the index is out of bounds.
    pub fn index<I: SliceIndex<[T]>>(
        self,
        index: I,
    ) -> Seal<'a, <I as SliceIndex<[T]>>::Output> {
        let ptr = unsafe { Seal::unseal_unchecked(self) };
        Seal::new(&mut ptr[index])
    }
}
