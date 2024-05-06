//! Validators that can check archived types.

mod archive;
mod shared;

use core::{any::TypeId, ops::Range};

pub use archive::*;
pub use shared::*;

use crate::validation::{ArchiveContext, SharedContext};

/// The default validator.
#[derive(Debug)]
pub struct DefaultValidator<'a> {
    archive: ArchiveValidator<'a>,
    shared: SharedValidator,
}

impl<'a> DefaultValidator<'a> {
    /// Creates a new validator from a byte range.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            archive: ArchiveValidator::new(bytes),
            shared: SharedValidator::new(),
        }
    }

    /// Create a new validator from a byte range with specific capacity.
    #[inline]
    pub fn with_capacity(bytes: &'a [u8], capacity: usize) -> Self {
        Self {
            archive: ArchiveValidator::new(bytes),
            shared: SharedValidator::with_capacity(capacity),
        }
    }
}

unsafe impl<'a, E> ArchiveContext<E> for DefaultValidator<'a>
where
    ArchiveValidator<'a>: ArchiveContext<E>,
{
    #[inline]
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &core::alloc::Layout,
    ) -> Result<(), E> {
        self.archive.check_subtree_ptr(ptr, layout)
    }

    #[inline]
    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        // SAFETY: This just forwards the call to the underlying
        // `ArchiveValidator`, which has the same safety requirements.
        unsafe { self.archive.push_subtree_range(root, end) }
    }

    #[inline]
    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        // SAFETY: This just forwards the call to the underlying
        // `ArchiveValidator`, which has the same safety requirements.
        unsafe { self.archive.pop_subtree_range(range) }
    }
}

impl<E> SharedContext<E> for DefaultValidator<'_>
where
    SharedValidator: SharedContext<E>,
{
    #[inline]
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E> {
        self.shared.register_shared_ptr(address, type_id)
    }
}
