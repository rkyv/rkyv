//! Validators that can check archived types.

mod archive;
mod shared;

use crate::validation::{ArchiveContext, SharedContext};
pub use archive::*;
use core::{any::TypeId, ops::Range};
pub use shared::*;

/// The default validator.
#[derive(Debug)]
pub struct DefaultValidator {
    archive: ArchiveValidator,
    shared: SharedValidator,
}

impl DefaultValidator {
    /// Creates a new validator from a byte range.
    #[inline]
    pub fn new(bytes: &[u8]) -> Self {
        Self {
            archive: ArchiveValidator::new(bytes),
            shared: SharedValidator::new(),
        }
    }

    /// Create a new validator from a byte range with specific capacity.
    #[inline]
    pub fn with_capacity(bytes: &[u8], capacity: usize) -> Self {
        Self {
            archive: ArchiveValidator::new(bytes),
            shared: SharedValidator::with_capacity(capacity),
        }
    }
}

unsafe impl<E> ArchiveContext<E> for DefaultValidator
where
    ArchiveValidator: ArchiveContext<E>,
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
    unsafe fn push_prefix_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        self.archive.push_prefix_subtree_range(root, end)
    }

    #[inline]
    unsafe fn push_suffix_subtree_range(
        &mut self,
        start: *const u8,
        root: *const u8,
    ) -> Result<Range<usize>, E> {
        self.archive.push_suffix_subtree_range(start, root)
    }

    #[inline]
    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        unsafe { self.archive.pop_subtree_range(range) }
    }
}

impl<E> SharedContext<E> for DefaultValidator
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
