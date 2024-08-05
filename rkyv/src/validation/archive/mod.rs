//! Basic archive buffer validation.

mod validator;

use core::{alloc::Layout, ops::Range};

use bytecheck::rancor::{Fallible, Source, Strategy};
use rancor::ResultExt as _;

pub use self::validator::*;
use crate::traits::LayoutRaw;

/// A context that can validate nonlocal archive memory.
///
/// # Safety
///
/// `check_subtree_ptr` must only return true if `ptr` is located entirely
/// within the subtree range and is safe to dereference.
pub unsafe trait ArchiveContext<E = <Self as Fallible>::Error> {
    /// Checks that the given data address and layout is located completely
    /// within the subtree range.
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), E>;

    /// Pushes a new subtree range onto the validator and starts validating it.
    ///
    /// After calling `push_subtree_range`, the validator will have a subtree
    /// range starting at the original start and ending at `root`. After popping
    /// the returned range, the validator will have a subtree range starting at
    /// `end` and ending at the original end.
    ///
    /// # Safety
    ///
    /// `root` and `end` must be located inside the archive.
    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E>;

    /// Pops the given range, restoring the original state with the pushed range
    /// removed.
    ///
    /// If the range was not popped in reverse order, an error is returned.
    ///
    /// # Safety
    ///
    /// `range` must be a range returned from this validator.
    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E>;
}

unsafe impl<T, E> ArchiveContext<E> for Strategy<T, E>
where
    T: ArchiveContext<E> + ?Sized,
{
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), E> {
        T::check_subtree_ptr(self, ptr, layout)
    }

    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        // SAFETY: This just forwards the call to the underlying context, which
        // has the same safety requirements.
        unsafe { T::push_subtree_range(self, root, end) }
    }

    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        // SAFETY: This just forwards the call to the underlying context, which
        // has the same safety requirements.
        unsafe { T::pop_subtree_range(self, range) }
    }
}

/// Helper methods for [`ArchiveContext`].
pub trait ArchiveContextExt<E>: ArchiveContext<E> {
    /// Checks that the given pointer and layout are within the current subtree
    /// range of the context, then pushes a new subtree range onto the validator
    /// for it and calls the given function.
    fn in_subtree_raw<R>(
        &mut self,
        ptr: *const u8,
        layout: Layout,
        f: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E>;

    /// Checks that the value the given pointer points to is within the current
    /// subtree range of the context, then pushes a new subtree range onto the
    /// validator for it and calls the given function.
    fn in_subtree<T: LayoutRaw + ?Sized, R>(
        &mut self,
        ptr: *const T,
        f: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E>;
}

impl<C: ArchiveContext<E> + ?Sized, E: Source> ArchiveContextExt<E> for C {
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn in_subtree_raw<R>(
        &mut self,
        ptr: *const u8,
        layout: Layout,
        f: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E> {
        self.check_subtree_ptr(ptr, &layout)?;

        // SAFETY: We checked that the entire range from `ptr` to
        // `ptr + layout.size()` is located within the buffer.
        let range =
            unsafe { self.push_subtree_range(ptr, ptr.add(layout.size()))? };

        let result = f(self)?;

        // SAFETY: `range` was returned from `push_subtree_range`.
        unsafe {
            self.pop_subtree_range(range)?;
        }

        Ok(result)
    }

    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    fn in_subtree<T: LayoutRaw + ?Sized, R>(
        &mut self,
        ptr: *const T,
        f: impl FnOnce(&mut Self) -> Result<R, E>,
    ) -> Result<R, E> {
        let layout = T::layout_raw(ptr_meta::metadata(ptr)).into_error()?;
        let root = ptr as *const u8;

        self.in_subtree_raw(root, layout, f)
    }
}
