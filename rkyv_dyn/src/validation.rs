//! Validation implementations and helper types.

use crate::{ArchivedDynMetadata, RegisteredImpl};
use bytecheck::{CheckBytes, rancor::{Fallible, fail, Error}, Verify};
use core::{
    alloc::Layout,
    fmt,
    marker::PhantomData,
};
use rkyv::validation::ArchiveContext;
use rkyv_typename::TypeName;
use std::collections::HashMap;

// This error just always says that check bytes isn't implemented for a type
#[derive(Debug)]
struct CheckBytesUnimplemented;

impl fmt::Display for CheckBytesUnimplemented {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "check bytes is not implemented for this type")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CheckBytesUnimplemented {}

type CheckBytesDyn<E> = unsafe fn(
    *const u8,
    &mut dyn ArchiveContext<E>,
) -> Result<(), E>;

// This is the fallback function that gets called if the archived type doesn't implement CheckBytes.
#[inline]
unsafe fn check_bytes_dyn_unimplemented<E: Error>(
    _bytes: *const u8,
    _context: &mut dyn ArchiveContext<E>,
) -> Result<(), E> {
    fail!(CheckBytesUnimplemented);
}

#[doc(hidden)]
pub trait NotCheckBytesDyn<E> {
    const CHECK_BYTES_DYN: CheckBytesDyn<E>;
}

impl<T: ?Sized, E: Error> NotCheckBytesDyn<E> for T {
    const CHECK_BYTES_DYN: CheckBytesDyn<E> = check_bytes_dyn_unimplemented::<E>;
}

#[doc(hidden)]
pub struct IsCheckBytesDyn<T: ?Sized, E>(PhantomData<(E, T)>);

impl<T, E> IsCheckBytesDyn<T, E>
where
    T: for<'a> CheckBytes<dyn ArchiveContext<E> + 'a>,
{
    #[doc(hidden)]
    pub const CHECK_BYTES_DYN: CheckBytesDyn<E> = Self::check_bytes_dyn;

    #[inline]
    unsafe fn check_bytes_dyn(
        bytes: *const u8,
        context: &mut dyn ArchiveContext<E>,
    ) -> Result<(), E> {
        T::check_bytes(bytes.cast(), context)?;
        Ok(())
    }
}

#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplValidation<E> {
    pub layout: Layout,
    pub check_bytes_dyn: CheckBytesDyn<E>,
}

#[doc(hidden)]
#[macro_export]
macro_rules! validation {
    ($type:ty as $trait:ty) => {
        use rkyv_dyn::validation::{
            ImplValidation, IsCheckBytesDyn, NotCheckBytesDyn,
        };
    };
}

#[derive(Debug)]
struct InvalidImplId {
    type_id: u64,
}

impl fmt::Display for InvalidImplId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid impl id: {} not registered", self.type_id)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidImplId {}

#[derive(Debug)]
struct MismatchedCachedVtable {
    type_id: u64,
    expected: usize,
    found: usize,
}

impl fmt::Display for MismatchedCachedVtable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mismatched cached vtable for {}: expected {} but found {}",
            self.type_id, self.expected, self.found
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MismatchedCachedVtable {}

unsafe impl<T, C> Verify<C> for ArchivedDynMetadata<T>
where
    T: TypeName + ?Sized,
    C: Fallible + ?Sized,
    C::Error: Error,
{
    fn verify(&self, context: &mut C) -> Result<(), C::Error> {
        if let Some(_) = IMPL_REGISTRY.get::<T>(self.type_id.to_native()) {
            Ok(())
        } else {
            fail!(InvalidImplId {
                type_id: self.type_id.to_native(),
            });
        }
    }
}

#[derive(Debug)]
struct InvalidMetadata {
    metadata: u64,
}

impl fmt::Display for InvalidMetadata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid metadata: {}", self.metadata)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidMetadata {}

#[doc(hidden)]
pub struct CheckBytesEntry<E> {
    vtable: usize,
    validation: ImplValidation<E>,
}

impl<E> CheckBytesEntry<E> {
    #[doc(hidden)]
    pub fn new<Ty, Tr>(check_bytes_dyn: CheckBytesDyn<E>) -> Self
    where
        Ty: RegisteredImpl<Tr>,
        Tr: ?Sized,
    {
        Self {
            vtable: <Ty as RegisteredImpl<Tr>>::vtable(),
            validation: ImplValidation {
                layout: Layout::new::<Ty>(),
                check_bytes_dyn,
            },
        }
    }
}

#[doc(hidden)]
pub struct CheckBytesRegistry<E> {
    vtable_to_check_bytes: HashMap<usize, ImplValidation<E>>,
}

impl<E> CheckBytesRegistry<E> {
    fn new() -> Self {
        Self {
            vtable_to_check_bytes: HashMap::new(),
        }
    }

    fn add_entry(&mut self, entry: &CheckBytesEntry<E>) {
        let old_value = self
            .vtable_to_check_bytes
            .insert(entry.vtable, entry.validation);

        debug_assert!(old_value.is_none(), "vtable conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    #[doc(hidden)]
    pub fn get(&self, vtable: usize) -> Option<&ImplValidation<E>> {
        self.vtable_to_check_bytes.get(&vtable)
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! register_validation {
    ($type:ty as $trait:ty) => {
        use rkyv_dyn::validation::{CheckBytesEntry, IsCheckBytesDyn, NotCheckBytesDyn};

        inventory::submit! { CheckBytesEntry::new::<$type, $trait>(IsCheckBytesDyn::<$type>::CHECK_BYTES_DYN) }
    }
}
