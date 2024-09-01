//! Validation implementations and helper types.

pub mod archive;
pub mod shared;

use core::{any::TypeId, ops::Range};

pub use self::{
    archive::{ArchiveContext, ArchiveContextExt},
    shared::SharedContext,
};

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
        address: usize,
        type_id: TypeId,
    ) -> Result<shared::ValidationState, E> {
        self.shared.start_shared(address, type_id)
    }

    fn finish_shared(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<(), E> {
        self.shared.finish_shared(address, type_id)
    }
}

#[cfg(test)]
mod tests {
    use rancor::Failure;

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

        let result = access::<ArchivedOption<ArchivedBox<[u8]>>, Failure>(
            &*synthetic_buf,
        );
        result.unwrap();

        // Out of bounds
        access_pos::<Archived<u32>, Failure>(&*Align([0, 1, 2, 3, 4]), 8)
            .expect_err("expected out of bounds error");
        // Overrun
        access_pos::<Archived<u32>, Failure>(&*Align([0, 1, 2, 3, 4]), 4)
            .expect_err("expected overrun error");
        // Unaligned
        access_pos::<Archived<u32>, Failure>(&*Align([0, 1, 2, 3, 4]), 1)
            .expect_err("expected unaligned error");
        // Underaligned
        access_pos::<Archived<u32>, Failure>(&Align([0, 1, 2, 3, 4])[1..], 0)
            .expect_err("expected underaligned error");
        // Undersized
        access::<Archived<u32>, Failure>(&*Align([]))
            .expect_err("expected out of bounds error");
    }

    #[cfg(feature = "pointer_width_32")]
    #[test]
    fn invalid_tags() {
        // Invalid archive (invalid tag)
        let synthetic_buf = Align([
            2u8, 0u8, 0u8, 0u8, // invalid tag + padding
            8u8, 0u8, 0u8, 0u8, // points 8 bytes forward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        let result = access_pos::<Archived<Option<Box<[u8]>>>, Failure>(
            &*synthetic_buf,
            0,
        );
        result.unwrap_err();
    }

    #[cfg(feature = "pointer_width_32")]
    #[test]
    fn overlapping_claims() {
        // Invalid archive (overlapping claims)
        let synthetic_buf = Align([
            // First string
            16u8, 0u8, 0u8, 0u8, // points 16 bytes forward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
            // Second string
            8u8, 0u8, 0u8, 0u8, // points 8 bytes forward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
        ]);

        access_pos::<Archived<[Box<[u8]>; 2]>, Failure>(&*synthetic_buf, 0)
            .unwrap_err();
    }

    #[cfg(feature = "pointer_width_32")]
    #[test]
    fn cycle_detection() {
        use bytecheck::CheckBytes;
        use rancor::{Fallible, Source};

        use crate::{
            ser::Writer, validation::ArchiveContext, Archive, Serialize,
        };

        #[allow(dead_code)]
        #[derive(Archive)]
        #[rkyv(crate, derive(Debug))]
        enum Node {
            Nil,
            Cons(#[omit_bounds] Box<Node>),
        }

        impl<S: Fallible + Writer + ?Sized> Serialize<S> for Node {
            fn serialize(
                &self,
                serializer: &mut S,
            ) -> Result<NodeResolver, S::Error> {
                Ok(match self {
                    Node::Nil => NodeResolver::Nil,
                    Node::Cons(inner) => {
                        NodeResolver::Cons(inner.serialize(serializer)?)
                    }
                })
            }
        }

        unsafe impl<C> CheckBytes<C> for ArchivedNode
        where
            C: Fallible + ArchiveContext + ?Sized,
            C::Error: Source,
        {
            unsafe fn check_bytes(
                value: *const Self,
                context: &mut C,
            ) -> Result<(), C::Error> {
                let bytes = value.cast::<u8>();
                let tag = unsafe { *bytes };
                match tag {
                    0 => (),
                    1 => unsafe {
                        <Archived<Box<Node>> as CheckBytes<C>>::check_bytes(
                            bytes.add(4).cast(),
                            context,
                        )?;
                    },
                    _ => panic!(),
                }
                Ok(())
            }
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

        access_pos::<ArchivedNode, Failure>(&*synthetic_buf, 0).unwrap_err();
    }
}
