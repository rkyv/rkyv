//! Validation implementation for ArchivedHashIndex.

use crate::{
    collections::ArchivedHashIndex,
    primitive::{ArchivedU32, ArchivedUsize},
    validation::ArchiveContext,
    RelPtr,
};
use bytecheck::CheckBytes;
use core::{
    alloc::{Layout, LayoutError},
    fmt, ptr,
};

/// Errors that can occur while checking an archived hash index.
#[derive(Debug)]
pub enum HashIndexError {
    /// An error occurred while checking the layouts of displacements or entries
    LayoutError(LayoutError),
    /// A displacement value was invalid
    InvalidDisplacement {
        /// The index of the entry with an invalid displacement
        index: usize,
        /// The value of the entry at the invalid location
        value: u32,
    },
}

impl fmt::Display for HashIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashIndexError::LayoutError(e) => write!(f, "layout error: {}", e),
            HashIndexError::InvalidDisplacement { index, value } => write!(
                f,
                "invalid displacement: value {} at index {}",
                value, index,
            ),
        }
    }
}

#[cfg(feature = "std")]
const _: () = {
    use std::error::Error;

    impl Error for HashIndexError {
        fn source(&self) -> Option<&(dyn Error + 'static)> {
            match self {
                HashIndexError::LayoutError(e) => Some(e as &dyn Error),
                HashIndexError::InvalidDisplacement { .. } => None,
            }
        }
    }
};

unsafe impl<C: ArchiveContext<E> + ?Sized, E> CheckBytes<C, E> for ArchivedHashIndex {
    unsafe fn check_bytes(
        value: *const Self,
        context: &mut C,
    ) -> Result<(), E> {
        let len =
            ArchivedUsize::check_bytes(ptr::addr_of!((*value).len), context)?
                .to_native() as usize;
        Layout::array::<ArchivedU32>(len)?;

        let displace_rel_ptr = RelPtr::manual_check_bytes(
            ptr::addr_of!((*value).displace),
            context,
        )?;
        let displace_ptr = context
            .check_subtree_ptr::<[ArchivedU32]>(
                displace_rel_ptr.base(),
                displace_rel_ptr.offset(),
                len,
            )
            .map_err(HashIndexError::ContextError)?;

        let range = context
            .push_prefix_subtree(displace_ptr)
            .map_err(HashIndexError::ContextError)?;
        let displace = <[ArchivedU32]>::check_bytes(displace_ptr, context)?;
        context
            .pop_prefix_range(range)
            .map_err(HashIndexError::ContextError)?;

        for (i, &d) in displace.iter().enumerate() {
            let d = d.to_native();
            if d as usize >= len && d < 0x80_00_00_00 {
                return Err(HashIndexError::InvalidDisplacement {
                    index: i,
                    value: d,
                });
            }
        }

        Ok(&*value)
    }
}
