//! Validation implementations and helper types.

pub mod archive;
pub mod shared;

use core::{any::TypeId, ops::Range};

pub use self::{
    archive::{ArchiveContext, ArchiveContextExt},
    shared::SharedContext,
};
use crate::erased::{ErasedPtr, Metadata};

/// The default validator.
#[derive(Debug)]
pub struct Validator<A, S> {
    archive: A,
    shared: S,
}

impl<A, S> Validator<A, S> {
    /// Creates a new validator from a byte range.
    #[inline]
    pub fn new(archive: A, shared: S) -> Self {
        Self { archive, shared }
    }
}

unsafe impl<A, S, E> ArchiveContext<E> for Validator<A, S>
where
    A: ArchiveContext<E>,
{
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &core::alloc::Layout,
    ) -> Result<(), E> {
        self.archive.check_subtree_ptr(ptr, layout)
    }

    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        // SAFETY: This just forwards the call to the underlying `CoreValidator`
        // which has the same safety requirements.
        unsafe { self.archive.push_subtree_range(root, end) }
    }

    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        // SAFETY: This just forwards the call to the underlying `CoreValidator`
        // which has the same safety requirements.
        unsafe { self.archive.pop_subtree_range(range) }
    }
}

impl<A, S, E> SharedContext<E> for Validator<A, S>
where
    S: SharedContext<E>,
{
    fn start_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
        metadata_is_eq: unsafe fn(Metadata, Metadata) -> bool,
    ) -> Result<shared::ValidationState, E> {
        self.shared
            .start_shared(shared_type_id, ptr, metadata_is_eq)
    }

    fn finish_shared(
        &mut self,
        shared_type_id: TypeId,
        ptr: ErasedPtr,
    ) -> Result<(), E> {
        self.shared.finish_shared(shared_type_id, ptr)
    }
}
