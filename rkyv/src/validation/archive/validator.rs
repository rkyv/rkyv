use core::{
    alloc::Layout, error::Error, fmt, marker::PhantomData, num::NonZeroUsize,
    ops::Range,
};

use rancor::{fail, OptionExt, Source};

use crate::validation::ArchiveContext;

const PTR_WIDTH: usize = (usize::BITS / 4 + 2) as usize;

struct Pointer(pub usize);

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#0w$x}", self.0, w = PTR_WIDTH)
    }
}

#[derive(Debug)]
struct UnalignedPointer {
    address: usize,
    align: usize,
}

impl fmt::Display for UnalignedPointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unaligned pointer: ptr {} unaligned for alignment {}",
            Pointer(self.address),
            self.align,
        )
    }
}

impl Error for UnalignedPointer {}

#[derive(Debug)]
struct InvalidSubtreePointer {
    address: usize,
    size: usize,
    subtree_range: Range<usize>,
}

impl fmt::Display for InvalidSubtreePointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "subtree pointer overran range: ptr {} size {} in range {}..{}",
            Pointer(self.address),
            self.size,
            Pointer(self.subtree_range.start),
            Pointer(self.subtree_range.end),
        )
    }
}

impl Error for InvalidSubtreePointer {}

#[derive(Debug)]
struct ExceededMaximumSubtreeDepth;

impl fmt::Display for ExceededMaximumSubtreeDepth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pushed a subtree range that exceeded the maximum subtree depth",
        )
    }
}

impl Error for ExceededMaximumSubtreeDepth {}

#[derive(Debug)]
struct RangePoppedTooManyTimes;

impl fmt::Display for RangePoppedTooManyTimes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subtree range popped too many times")
    }
}

impl Error for RangePoppedTooManyTimes {}

#[derive(Debug)]
struct RangePoppedOutOfOrder;

impl fmt::Display for RangePoppedOutOfOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "subtree range popped out of order")
    }
}

impl Error for RangePoppedOutOfOrder {}

/// A validator that can verify archives with nonlocal memory.
#[derive(Debug)]
pub struct ArchiveValidator<'a> {
    subtree_range: Range<usize>,
    max_subtree_depth: Option<NonZeroUsize>,
    _phantom: PhantomData<&'a [u8]>,
}

impl<'a> ArchiveValidator<'a> {
    /// Creates a new bounds validator for the given bytes.
    #[inline]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self::with_max_depth(bytes, None)
    }

    /// Crates a new bounds validator for the given bytes with a maximum
    /// validation depth.
    #[inline]
    pub fn with_max_depth(
        bytes: &'a [u8],
        max_subtree_depth: Option<NonZeroUsize>,
    ) -> Self {
        let Range { start, end } = bytes.as_ptr_range();
        Self {
            subtree_range: Range {
                start: start as usize,
                end: end as usize,
            },
            max_subtree_depth,
            _phantom: PhantomData,
        }
    }
}

unsafe impl<E: Source> ArchiveContext<E> for ArchiveValidator<'_> {
    fn check_subtree_ptr(
        &mut self,
        ptr: *const u8,
        layout: &Layout,
    ) -> Result<(), E> {
        let start = ptr as usize;
        let end = ptr.wrapping_add(layout.size()) as usize;
        if end < start
            || start < self.subtree_range.start
            || end > self.subtree_range.end
        {
            fail!(InvalidSubtreePointer {
                address: start,
                size: layout.size(),
                subtree_range: self.subtree_range.clone(),
            });
        } else if start & (layout.align() - 1) != 0 {
            fail!(UnalignedPointer {
                address: ptr as usize,
                align: layout.align(),
            });
        } else {
            Ok(())
        }
    }

    unsafe fn push_subtree_range(
        &mut self,
        root: *const u8,
        end: *const u8,
    ) -> Result<Range<usize>, E> {
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = NonZeroUsize::new(max_subtree_depth.get() - 1)
                .into_trace(ExceededMaximumSubtreeDepth)?;
        }

        let result = Range {
            start: end as usize,
            end: self.subtree_range.end,
        };
        self.subtree_range.end = root as usize;
        Ok(result)
    }

    unsafe fn pop_subtree_range(
        &mut self,
        range: Range<usize>,
    ) -> Result<(), E> {
        if range.start < self.subtree_range.end {
            fail!(RangePoppedOutOfOrder);
        }
        self.subtree_range = range;
        if let Some(max_subtree_depth) = &mut self.max_subtree_depth {
            *max_subtree_depth = max_subtree_depth
                .checked_add(1)
                .into_trace(RangePoppedTooManyTimes)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use rancor::Error;

    use super::*;
    use crate::{
        api::low::{access, access_pos},
        boxed::ArchivedBox,
        option::ArchivedOption,
        util::Align,
        Archived,
    };

    #[test]
    fn basic_functionality() {
        #[cfg(all(feature = "pointer_width_16", not(feature = "big_endian")))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // padding to 2-alignment
            1u8, 0u8, // Some + padding
            0xf2u8, 0xffu8, // points 14 bytes backwards
            11u8, 0u8, // string is 11 characters long
        ]);

        #[cfg(all(feature = "pointer_width_16", feature = "big_endian"))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // padding to 2-alignment
            1u8, 0u8, // Some + padding
            0xffu8, 0xf2u8, // points 14 bytes backwards
            0u8, 11u8, // string is 11 characters long
        ]);

        #[cfg(all(
            not(any(
                feature = "pointer_width_16",
                feature = "pointer_width_64",
            )),
            not(feature = "big_endian"),
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // padding to 4-alignment
            1u8, 0u8, 0u8, 0u8, // Some + padding
            0xf0u8, 0xffu8, 0xffu8, 0xffu8, // points 16 bytes backward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        ]);

        #[cfg(all(
            not(any(
                feature = "pointer_width_16",
                feature = "pointer_width_64",
            )),
            feature = "big_endian",
        ))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // padding to 4-alignment
            1u8, 0u8, 0u8, 0u8, // Some + padding
            0xffu8, 0xffu8, 0xffu8, 0xf0u8, // points 16 bytes backward
            0u8, 0u8, 0u8, 11u8, // string is 11 characters long
        ]);

        #[cfg(all(feature = "pointer_width_64", not(feature = "big_endian")))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, 0u8, 0u8, 0u8, 0u8, // padding to 8-alignment
            1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, // Some + padding
            // points 24 bytes backward
            0xe8u8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8,
            11u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, // string is 11 characters long
        ]);

        #[cfg(all(feature = "pointer_width_64", feature = "big_endian"))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world!!!!!" because otherwise the string will get inlined
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0x21, 0x21, 0x21, 0x21, 0x21, 1u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            0u8, // Some + padding
            // points 24 bytes backward
            0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xffu8, 0xe8u8, 0u8,
            0u8, 0u8, 0u8, 0u8, 0u8, 0u8,
            11u8, // string is 11 characters long
        ]);

        let result =
            access::<ArchivedOption<ArchivedBox<[u8]>>, Error>(&*synthetic_buf);
        result.unwrap();

        // Out of bounds
        let result =
            access_pos::<Archived<u32>, Error>(&*Align([0, 1, 2, 3, 4]), 8);
        assert_source!(
            result.unwrap_err(),
            InvalidSubtreePointer { size: 4, .. },
            "error source should be out-of-bounds",
        );
        // Overrun
        let result =
            access_pos::<Archived<u32>, Error>(&*Align([0, 1, 2, 3, 4]), 4);
        assert_source!(
            result.unwrap_err(),
            InvalidSubtreePointer { size: 4, .. },
            "error source should be overrun",
        );
        // Unaligned
        let result =
            access_pos::<Archived<u32>, Error>(&*Align([0, 1, 2, 3, 4]), 1);
        assert_source!(
            result.unwrap_err(),
            UnalignedPointer { align: 4, .. },
            "error source should be unaligned",
        );
    }

    #[cfg(not(any(
        feature = "pointer_width_16",
        feature = "pointer_width_64"
    )))]
    #[test]
    fn invalid_tags() {
        // Invalid archive (invalid tag)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // pad to 4-alignment
            2u8, 0u8, 0u8, 0u8, // invalid tag + padding
            0xe8, 0xff, 0xff, 0xff, // points 24 bytes backward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        ]);

        let result =
            access::<Archived<Option<Box<[u8]>>>, Error>(&*synthetic_buf);
        assert_source!(
            result.unwrap_err(),
            bytecheck::InvalidEnumDiscriminantError::<u8> {
                enum_name: "ArchivedOption",
                invalid_discriminant: 2,
            },
            "error source should be invalid enum discriminant",
        );
    }

    #[cfg(not(any(
        feature = "pointer_width_16",
        feature = "pointer_width_64"
    )))]
    #[test]
    fn overlapping_claims() {
        // Invalid archive (overlapping claims)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0, // pad to 4-alignment
            // First string
            0xf0, 0xff, 0xff, 0xff, // points 16 bytes backward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
            // Second string
            0xe8, 0xff, 0xff, 0xff, // points 24 bytes forward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        ]);

        let result = access::<Archived<[Box<[u8]>; 2]>, Error>(&*synthetic_buf);
        assert_source!(
            result.unwrap_err(),
            InvalidSubtreePointer { size: 11, .. },
            "error source should be invalid subtree pointer",
        );
    }

    #[cfg(not(any(
        feature = "pointer_width_16",
        feature = "pointer_width_64"
    )))]
    #[test]
    fn cycle_detection() {
        use crate::{
            ser::Writer, validation::ArchiveContext, Archive, Serialize,
        };

        #[allow(dead_code)]
        #[derive(Archive, Serialize)]
        #[rkyv(
            crate,
            serialize_bounds(__S: Writer),
            bytecheck(bounds(__C: ArchiveContext)),
            derive(Debug),
        )]
        enum Node {
            Nil,
            Cons(#[rkyv(omit_bounds)] Box<Node>),
        }

        // Invalid archive (cyclic claims)
        let synthetic_buf = Align([
            // First node
            1u8, 0u8, 0u8, 0u8, // Cons
            4u8, 0u8, 0u8, 0u8, // Node is 4 bytes forward
            // Second string
            1u8, 0u8, 0u8, 0u8, // Cons
            244u8, 255u8, 255u8, 255u8, // Node is 12 bytes back
        ]);

        let result = access::<ArchivedNode, Error>(&*synthetic_buf);
        assert_source!(
            result.unwrap_err(),
            InvalidSubtreePointer { .. },
            "error source should be invalid subtree pointer",
        );
    }
}
