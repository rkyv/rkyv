//! Validators add validation capabilities by wrapping and extending basic validators.

use crate::{
    validation::{
        check_archived_root_with_context, check_archived_value_with_context,
        ArchiveContext, ArchivePrefixRange, ArchiveSuffixRange, CheckTypeError, LayoutRaw,
        SharedArchiveContext,
    },
    Archive,
    Fallible,
};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;
use bytecheck::CheckBytes;
use core::{any::TypeId, fmt, ops::Range};
use ptr_meta::Pointee;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use hashbrown::HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

/// Errors that can occur when checking a relative pointer
#[derive(Debug)]
pub enum ArchiveError {
    /// Computing the target of a relative pointer overflowed
    Overflow {
        /// The base pointer
        base: *const u8,
        /// The offset
        offset: isize,
    },
    /// The archive is under-aligned for one of the types inside
    Underaligned {
        /// The expected alignment of the archive
        expected_align: usize,
        /// The actual alignment of the archive
        actual_align: usize,
    },
    /// A pointer pointed outside the bounds of the archive
    OutOfBounds {
        /// The base of the relative pointer
        base: *const u8,
        /// The offset of the relative pointer
        offset: isize,
        /// The pointer range of the archive
        range: Range<*const u8>,
    },
    /// There wasn't enough space for the desired type at the pointed location
    Overrun {
        /// The pointer to the type
        ptr: *const u8,
        /// The desired size of the type
        size: usize,
        /// The pointer range of the archive
        range: Range<*const u8>,
    },
    /// The pointer wasn't aligned properly for the desired type
    Unaligned {
        /// The pointer to the type
        ptr: *const u8,
        /// The required alignment of the type
        align: usize,
    },
    /// The pointer wasn't within the subtree range
    SubtreePointerOutOfBounds {
        /// The pointer to the subtree
        ptr: *const u8,
        /// The subtree range
        subtree_range: Range<*const u8>,
    },
    /// There wasn't enough space in the subtree range for the desired type at the pointed location
    SubtreePointerOverrun {
        /// The pointer to the subtree type,
        ptr: *const u8,
        /// The desired size of the type
        size: usize,
        /// The subtree range
        subtree_range: Range<*const u8>,
    },
    /// A subtree range was popped out of order.
    ///
    /// Subtree ranges must be popped in the reverse of the order they are pushed.
    RangePoppedOutOfOrder {
        /// The expected depth of the range
        expected_depth: usize,
        /// The actual depth of the range
        actual_depth: usize,
    },
    /// A subtree range was not popped before validation concluded
    UnpoppedSubtreeRanges {
        /// The depth of the last subtree that was pushed
        last_range: usize,
    }
}

impl fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchiveError::Overflow {
                base,
                offset,
            } => write!(
                f,
                "relative pointer overflowed: base {:p} offset {}",
                base, offset
            ),
            ArchiveError::Underaligned {
                expected_align,
                actual_align,
            } => write!(
                f,
                "archive underaligned: need alignment {} but have alignment {}",
                expected_align, actual_align
            ),
            ArchiveError::OutOfBounds {
                base,
                offset,
                range,
            } => write!(
                f,
                "pointer out of bounds: base {:p} offset {} not in range {:p}..{:p}",
                base, offset, range.start, range.end
            ),
            ArchiveError::Overrun {
                ptr,
                size,
                range,
            } => write!(
                f,
                "pointer overran buffer: ptr {:p} size {} in range {:p}..{:p}",
                ptr, size, range.start, range.end
            ),
            ArchiveError::Unaligned { ptr, align } => write!(
                f,
                "unaligned pointer: ptr {:p} unaligned for alignment {}",
                ptr, align
            ),
            ArchiveError::SubtreePointerOutOfBounds { ptr, subtree_range } => write!(
                f,
                "subtree pointer out of bounds: ptr {:p} not in range {:p}..{:p}",
                ptr, subtree_range.start, subtree_range.end
            ),
            ArchiveError::SubtreePointerOverrun { ptr, size, subtree_range } => write!(
                f,
                "subtree pointer overran range: ptr {:p} size {} in range {:p}..{:p}",
                ptr, size, subtree_range.start, subtree_range.end
            ),
            ArchiveError::RangePoppedOutOfOrder { expected_depth, actual_depth } => write!(
                f,
                "subtree range popped out of order: expected depth {}, actual depth {}",
                expected_depth, actual_depth
            ),
            ArchiveError::UnpoppedSubtreeRanges { last_range } => write!(
                f,
                "unpopped subtree ranges: last range {}",
                last_range
            ),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ArchiveError {}

/// A validator that can validate archives with nonlocal memory.
pub struct ArchiveValidator<'a> {
    bytes: &'a [u8],
    subtree_range: Range<*const u8>,
    subtree_depth: usize,
}

impl<'a> ArchiveValidator<'a> {
    /// Creates a new bounds validator for the given bytes.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            subtree_range: bytes.as_ptr_range(),
            subtree_depth: 0,
        }
    }

    /// Returns the log base 2 of the alignment of the archive.
    ///
    /// An archive that is 2-aligned will return 1, 4-aligned will return 2, 8-aligned will return 3
    /// and so on.
    #[inline]
    pub fn log_alignment(&self) -> usize {
        (self.bytes.as_ptr() as usize).trailing_zeros() as usize
    }

    /// Returns the alignment of the archive.
    #[inline]
    pub fn alignment(&self) -> usize {
        1 << self.log_alignment()
    }
}

impl<'a> From<&'a [u8]> for ArchiveValidator<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self::new(bytes)
    }
}

impl<'a> Fallible for ArchiveValidator<'a> {
    type Error = ArchiveError;
}

impl<'a> ArchiveContext for ArchiveValidator<'a> {
    unsafe fn check_ptr<T: LayoutRaw + Pointee + ?Sized>(
        &mut self,
        base: *const u8,
        offset: isize,
        metadata: T::Metadata,
    ) -> Result<*const T, Self::Error> {
        let base_pos = base.offset_from(self.bytes.as_ptr());
        let target_pos = base_pos.checked_add(offset)
            .ok_or(ArchiveError::Overflow {
                base,
                offset,
            })?;
        if target_pos < 0 || target_pos as usize > self.bytes.len() {
            Err(ArchiveError::OutOfBounds {
                base,
                offset,
                range: self.bytes.as_ptr_range(),
            })
        } else {
            let data_address = base.offset(offset);
            let ptr = ptr_meta::from_raw_parts(data_address as *const (), metadata);
            let layout = T::layout_raw(ptr);
            if self.alignment() < layout.align() {
                Err(ArchiveError::Underaligned {
                    expected_align: layout.align(),
                    actual_align: self.alignment(),
                })
            } else {
                if (data_address as usize) & (layout.align() - 1) != 0 {
                    Err(ArchiveError::Unaligned {
                        ptr: data_address,
                        align: layout.align(),
                    })
                } else if self.bytes.len() - (target_pos as usize) < layout.size() {
                    Err(ArchiveError::Overrun {
                        ptr: data_address,
                        size: layout.size(),
                        range: self.bytes.as_ptr_range(),
                    })
                } else {
                    Ok(ptr)
                }
            }
        }
    }

    unsafe fn check_subtree_ptr_bounds<T: LayoutRaw + ?Sized>(&mut self, ptr: *const T) -> Result<(), Self::Error> {
        let subtree_ptr = ptr as *const u8;
        if !self.subtree_range.contains(&subtree_ptr) {
            Err(ArchiveError::SubtreePointerOutOfBounds {
                ptr: subtree_ptr,
                subtree_range: self.subtree_range.clone(),
            })
        } else {
            let available_space = self.subtree_range.end.offset_from(subtree_ptr) as usize;
            let layout = T::layout_raw(ptr);
            if available_space < layout.size() {
                Err(ArchiveError::SubtreePointerOverrun {
                    ptr: subtree_ptr,
                    size: layout.size(),
                    subtree_range: self.subtree_range.clone(),
                })
            } else {
                Ok(())
            }
        }
    }

    #[inline]
    unsafe fn push_prefix_subtree_range(&mut self, root: *const u8, end: *const u8) -> Result<ArchivePrefixRange, Self::Error> {
        let result = ArchivePrefixRange {
            range: Range {
                start: end,
                end: self.subtree_range.end,
            },
            depth: self.subtree_depth,
        };
        self.subtree_depth += 1;
        self.subtree_range.end = root;
        Ok(result)
    }

    #[inline]
    fn pop_prefix_range(&mut self, range: ArchivePrefixRange) -> Result<(), Self::Error> {
        if self.subtree_depth - 1 != range.depth {
            Err(ArchiveError::RangePoppedOutOfOrder {
                expected_depth: self.subtree_depth - 1,
                actual_depth: range.depth,
            })
        } else {
            self.subtree_range = range.range;
            self.subtree_depth = range.depth;
            Ok(())
        }
    }

    #[inline]
    unsafe fn push_suffix_subtree_range(&mut self, start: *const u8, root: *const u8) -> Result<ArchiveSuffixRange, Self::Error> {
        let result = ArchiveSuffixRange {
            start: self.subtree_range.start,
            depth: self.subtree_depth,
        };
        self.subtree_depth += 1;
        self.subtree_range.start = start;
        self.subtree_range.end = root;
        Ok(result)
    }

    #[inline]
    fn pop_suffix_range(&mut self, range: ArchiveSuffixRange) -> Result<(), Self::Error> {
        if self.subtree_depth - 1 != range.depth {
            Err(ArchiveError::RangePoppedOutOfOrder {
                expected_depth: self.subtree_depth - 1,
                actual_depth: range.depth,
            })
        } else {
            self.subtree_range.end = self.subtree_range.start;
            self.subtree_range.start = range.start;
            self.subtree_depth = range.depth;
            Ok(())
        }
    }

    #[inline]
    fn finish(&mut self) -> Result<(), Self::Error> {
        if self.subtree_depth != 0 {
            Err(ArchiveError::UnpoppedSubtreeRanges {
                last_range: self.subtree_depth - 1,
            })
        } else {
            Ok(())
        }
    }
}

/// Errors that can occur when checking shared memory.
#[derive(Debug)]
pub enum SharedArchiveError<E> {
    /// An error occurred while checking the memory of the archive
    Inner(E),
    /// Multiple pointers exist to the same location with different types
    TypeMismatch {
        /// A previous type that the location was checked as
        previous: TypeId,
        /// The current type that the location is checked as
        current: TypeId,
    },
}

impl<E: fmt::Display> fmt::Display for SharedArchiveError<E> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedArchiveError::Inner(e) => e.fmt(f),
            SharedArchiveError::TypeMismatch { previous, current } => write!(
                f,
                "the same memory region has been claimed as two different types ({:?} and {:?})",
                previous, current
            ),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl<E: Error + 'static> Error for SharedArchiveError<E> {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                SharedArchiveError::Inner(e) => Some(e as &dyn Error),
                SharedArchiveError::TypeMismatch { .. } => None,
            }
        }
    }
};

/// An adapter that adds shared memory validation.
pub struct SharedArchiveValidator<C> {
    inner: C,
    shared: HashMap<*const (), TypeId>,
}

impl<C> SharedArchiveValidator<C> {
    /// Wraps the given context and adds shared memory validation.
    #[inline]
    pub fn new(inner: C) -> Self {
        Self {
            inner,
            shared: HashMap::new(),
        }
    }

    /// Consumes the adapter and returns the underlying validator.
    #[inline]
    pub fn into_inner(self) -> C {
        self.inner
    }
}

impl<'a, C: From<&'a [u8]>> From<&'a [u8]> for SharedArchiveValidator<C> {
    fn from(bytes: &'a [u8]) -> Self {
        Self::new(C::from(bytes))
    }
}

impl<C: Fallible> Fallible for SharedArchiveValidator<C> {
    type Error = SharedArchiveError<C::Error>;
}

impl<C: ArchiveContext> ArchiveContext for SharedArchiveValidator<C> {
    #[inline]
    unsafe fn check_ptr<T: LayoutRaw + Pointee + ?Sized>(
        &mut self,
        base: *const u8,
        offset: isize,
        metadata: T::Metadata,
    ) -> Result<*const T, Self::Error> {
        self.inner.check_ptr(base, offset, metadata)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    unsafe fn check_subtree_ptr_bounds<T: LayoutRaw + ?Sized>(
        &mut self,
        ptr: *const T,
    ) -> Result<(), Self::Error> {
        self.inner.check_subtree_ptr_bounds(ptr)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    unsafe fn push_prefix_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<ArchivePrefixRange, Self::Error> {
        self.inner.push_prefix_subtree_range(root, end)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    fn pop_prefix_range(&mut self, range: ArchivePrefixRange) -> Result<(), Self::Error> {
        self.inner.pop_prefix_range(range)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    unsafe fn push_suffix_subtree_range(
        &mut self,
        start: *const u8,
        root: *const u8,
    ) -> Result<ArchiveSuffixRange, Self::Error> {
        self.inner.push_suffix_subtree_range(start, root)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    fn pop_suffix_range(&mut self, range: ArchiveSuffixRange) -> Result<(), Self::Error> {
        self.inner.pop_suffix_range(range)
            .map_err(SharedArchiveError::Inner)
    }

    #[inline]
    fn finish(&mut self) -> Result<(), Self::Error> {
        self.inner.finish()
            .map_err(SharedArchiveError::Inner)
    }
}

impl<C: ArchiveContext> SharedArchiveContext for SharedArchiveValidator<C> {
    unsafe fn check_shared_ptr<T: LayoutRaw + ?Sized>(
        &mut self,
        ptr: *const T,
        type_id: TypeId,
    ) -> Result<Option<*const T>, Self::Error> {
        let key = ptr as *const ();
        if let Some(previous_type_id) = self.shared.get(&key) {
            if previous_type_id != &type_id {
                Err(SharedArchiveError::TypeMismatch {
                    previous: *previous_type_id,
                    current: type_id,
                })
            } else {
                Ok(None)
            }
        } else {
            self.shared.insert(key, type_id);
            Ok(Some(ptr))
        }
    }
}

/// A validator that supports all builtin types.
pub type DefaultArchiveValidator<'a> = SharedArchiveValidator<ArchiveValidator<'a>>;

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
pub fn check_archived_value<'a, T: Archive>(
    bytes: &'a [u8],
    pos: usize,
) -> Result<&T::Archived, CheckTypeError<T::Archived, DefaultArchiveValidator<'a>>>
where
    T::Archived: CheckBytes<DefaultArchiveValidator<'a>>,
{
    let mut validator = DefaultArchiveValidator::from(bytes);
    check_archived_value_with_context::<T, DefaultArchiveValidator>(bytes, pos, &mut validator)
}

/// Checks the given archive at the given position for an archived version of the given type.
///
/// This is a safe alternative to [`archived_value`](crate::archived_value) for types that implement
/// `CheckBytes`.
///
/// See [`check_archived_value`] for more details.
#[inline]
pub fn check_archived_root<'a, T: Archive>(
    bytes: &'a [u8],
) -> Result<&T::Archived, CheckTypeError<T::Archived, DefaultArchiveValidator<'a>>>
where
    T::Archived: CheckBytes<DefaultArchiveValidator<'a>>,
{
    let mut validator = DefaultArchiveValidator::from(bytes);
    check_archived_root_with_context::<T, DefaultArchiveValidator>(bytes, &mut validator)
}
