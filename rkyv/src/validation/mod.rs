//! Validation implementations and helper types.

pub mod util;
pub mod validators;

use core::{alloc::Layout, any::TypeId, ops::Range};

use bytecheck::rancor::{Fallible, Source, Strategy};
use rancor::ResultExt as _;

use crate::LayoutRaw;

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

/// A context that can validate shared archive memory.
///
/// Shared pointers require this kind of context to validate.
pub trait SharedContext<E = <Self as Fallible>::Error> {
    /// Registers the given `ptr` as a shared pointer with the given type.
    ///
    /// Returns `true` if the pointer was newly-registered and `check_bytes`
    /// should be called.
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E>;
}

impl<T, E> SharedContext<E> for Strategy<T, E>
where
    T: SharedContext<E>,
{
    fn register_shared_ptr(
        &mut self,
        address: usize,
        type_id: TypeId,
    ) -> Result<bool, E> {
        T::register_shared_ptr(self, address, type_id)
    }
}

#[cfg(test)]
mod tests {
    use rancor::Failure;

    use crate::{access, util::Align, validation::util::access_pos, Archived};

    #[test]
    fn basic_functionality() {
        #[cfg(all(feature = "pointer_width_16", feature = "little_endian"))]
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

        #[cfg(all(feature = "pointer_width_32", feature = "little_endian"))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // padding to 4-alignment
            1u8, 0u8, 0u8, 0u8, // Some + padding
            0xf0u8, 0xffu8, 0xffu8, 0xffu8, // points 16 bytes backward
            11u8, 0u8, 0u8, 0u8, // string is 11 characters long
        ]);

        #[cfg(all(feature = "pointer_width_32", feature = "big_endian"))]
        // Synthetic archive (correct)
        let synthetic_buf = Align([
            // "Hello world"
            0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x77, 0x6f, 0x72, 0x6c, 0x64,
            0u8, // padding to 4-alignment
            1u8, 0u8, 0u8, 0u8, // Some + padding
            0xffu8, 0xffu8, 0xffu8, 0xf0u8, // points 16 bytes backward
            0u8, 0u8, 0u8, 11u8, // string is 11 characters long
        ]);

        #[cfg(all(feature = "pointer_width_64", feature = "little_endian"))]
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
            access::<Archived<Option<Box<[u8]>>>, Failure>(&*synthetic_buf);
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
        #[rkyv(crate)]
        #[rkyv_attr(derive(Debug))]
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
