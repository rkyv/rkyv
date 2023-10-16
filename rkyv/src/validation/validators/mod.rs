//! Validators that can check archived types.

mod archive;
mod shared;
mod util;

use crate::{
    validation::{
        check_archived_root_with_context, check_archived_value_with_context,
        ArchiveContext, SharedContext,
    },
    Archive,
};
pub use archive::*;
use bytecheck::{CheckBytes, rancor::Error};
use core::{
    any::TypeId,
    ops::Range,
};
pub use shared::*;
pub use util::*;

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
        unsafe {
            self.archive.pop_subtree_range(range)
        }
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

/// Checks the given archive at the given position for an archived version of the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// # Examples
/// ```
/// use rkyv::{
///     check_archived_value,
///     ser::{Serializer, serializers::AlignedSerializer},
///     AlignedVec,
///     Archive,
///     Serialize,
/// };
/// use bytecheck::CheckBytes;
///
/// #[derive(Archive, Serialize)]
/// #[archive_attr(derive(CheckBytes))]
/// struct Example {
///     name: String,
///     value: i32,
/// }
///
/// let value = Example {
///     name: "pi".to_string(),
///     value: 31415926,
/// };
///
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// let pos = serializer.serialize_value(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = check_archived_value::<Example>(buf.as_ref(), pos).unwrap();
/// ```
#[inline]
pub fn check_archived_value<T: Archive, E>(
    bytes: &[u8],
    pos: isize,
) -> Result<&T::Archived, E>
where
    T::Archived: CheckBytes<DefaultValidator, E>,
    E: Error,
{
    let mut validator = DefaultValidator::new(bytes);
    check_archived_value_with_context::<T, DefaultValidator, E>(
        bytes,
        pos,
        &mut validator,
    )
}

/// Checks the given archive at the given position for an archived version of the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// See [`check_archived_value`] for more details.
#[inline]
pub fn check_archived_root<T: Archive, E>(
    bytes: &[u8],
) -> Result<&T::Archived, E>
where
    T::Archived: CheckBytes<DefaultValidator, E>,
    E: Error,
{
    let mut validator = DefaultValidator::new(bytes);
    check_archived_root_with_context::<T, DefaultValidator, E>(
        bytes,
        &mut validator,
    )
}
