//! Validation implementations for shared pointers.

use core::{any::TypeId, fmt};
use std::error::Error;
use bytecheck::CheckBytes;
use ptr_meta::Pointee;
use super::{ArchivedArc, ArchivedRc};
use crate::{
    validation::{
        ArchiveBoundsContext,
        LayoutMetadata,
        SharedArchiveContext,
    },
    ArchivePointee,
    RelPtr,
};

/// Errors that can occur while checking archived shared pointers.
#[derive(Debug)]
pub enum SharedPointerError<T, R, C> {
    /// An error occurred while checking the bytes of a shared value
    PointerCheckBytesError(T),
    /// An error occurred while checking the bytes of a shared reference
    ValueCheckBytesError(R),
    /// A context error occurred
    ContextError(C),
}

impl<T: fmt::Display, R: fmt::Display, C: fmt::Display> fmt::Display for SharedPointerError<T, R, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedPointerError::PointerCheckBytesError(e) => e.fmt(f),
            SharedPointerError::ValueCheckBytesError(e) => e.fmt(f),
            SharedPointerError::ContextError(e) => e.fmt(f),
        }
    }
}

impl<T: Error + 'static, R: Error + 'static, C: Error + 'static> Error for SharedPointerError<T, R, C> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedPointerError::PointerCheckBytesError(e) => Some(e as &dyn Error),
            SharedPointerError::ValueCheckBytesError(e) => Some(e as &dyn Error),
            SharedPointerError::ContextError(e) => Some(e as &dyn Error),
        }
    }
}

impl<T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized + 'static, C: ArchiveBoundsContext + SharedArchiveContext + ?Sized> CheckBytes<C> for ArchivedRc<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error = SharedPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<T>::manual_check_bytes(value.cast(), context)
            .map_err(SharedPointerError::PointerCheckBytesError)?;
        if let Some(ptr) = context.claim_shared_ptr(rel_ptr, TypeId::of::<ArchivedRc<T>>()).map_err(SharedPointerError::ContextError)? {
            T::check_bytes(ptr, context)
                .map_err(SharedPointerError::ValueCheckBytesError)?;
        }
        Ok(&*value)
    }
}

impl<T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized + 'static, C: ArchiveBoundsContext + SharedArchiveContext + ?Sized> CheckBytes<C> for ArchivedArc<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error = SharedPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<T>::manual_check_bytes(value.cast(), context)
            .map_err(SharedPointerError::PointerCheckBytesError)?;
        if let Some(ptr) = context.claim_shared_ptr(rel_ptr, TypeId::of::<ArchivedArc<T>>()).map_err(SharedPointerError::ContextError)? {
            T::check_bytes(ptr, context)
                .map_err(SharedPointerError::ValueCheckBytesError)?;
        }
        Ok(&*value)
    }
}
