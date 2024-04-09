//! Validation implementations and helper types.

use crate::{ArchivedDynMetadata, register::Registered};
use bytecheck::{CheckBytes, rancor::{Fallible, fail, Error}, Verify};
use core::{
    alloc::Layout,
    fmt,
    marker::PhantomData,
};
use rkyv::validation::ArchiveContext;
use std::collections::HashMap;

#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplValidation<E> {
    pub layout: Layout,
    pub check_bytes: fn(*const (), &mut dyn ArchiveContext<E>) -> Result<(), E>,
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

unsafe impl<T, C> Verify<C> for ArchivedDynMetadata<T>
where
    T: ?Sized,
    C: Fallible + ?Sized,
    C::Error: Source,
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

