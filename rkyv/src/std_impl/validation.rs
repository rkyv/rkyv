//! Validation implementations for std types.

use super::{ArchivedBox, ArchivedString, ArchivedVec};
use crate::{
    validation::{ArchiveBoundsContext, ArchiveMemoryContext, LayoutMetadata},
    ArchivePointee, Fallible, RelPtr,
};
use bytecheck::CheckBytes;
use core::fmt;
use ptr_meta::Pointee;
use std::error::Error;

/// Errors that can occur while chechking archived owned pointers
#[derive(Debug)]
pub enum OwnedPointerError<T, R, C> {
    /// The pointer failed to validate due to invalid metadata.
    PointerCheckBytesError(T),
    /// The value pointed to by the owned pointer was invalid.
    ValueCheckBytesError(R),
    /// An error occurred from the validation context.
    ContextError(C),
}

impl<T: fmt::Display, R: fmt::Display, C: fmt::Display> fmt::Display
    for OwnedPointerError<T, R, C>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnedPointerError::PointerCheckBytesError(e) => e.fmt(f),
            OwnedPointerError::ValueCheckBytesError(e) => e.fmt(f),
            OwnedPointerError::ContextError(e) => e.fmt(f),
        }
    }
}

impl<T: Error + 'static, R: Error + 'static, C: Error + 'static> Error
    for OwnedPointerError<T, R, C>
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OwnedPointerError::PointerCheckBytesError(e) => Some(e as &dyn Error),
            OwnedPointerError::ValueCheckBytesError(e) => Some(e as &dyn Error),
            OwnedPointerError::ContextError(e) => Some(e as &dyn Error),
        }
    }
}

type CheckOwnedPointerError<T, C> = OwnedPointerError<
    <<T as ArchivePointee>::ArchivedMetadata as CheckBytes<C>>::Error,
    <T as CheckBytes<C>>::Error,
    <C as Fallible>::Error,
>;

impl<C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C> for ArchivedString
where
    C::Error: Error,
{
    type Error = CheckOwnedPointerError<str, C>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<str>::manual_check_bytes(value.cast(), context)
            .map_err(OwnedPointerError::PointerCheckBytesError)?;
        let ptr = context
            .claim_owned_rel_ptr(rel_ptr)
            .map_err(OwnedPointerError::ContextError)?;
        <str as CheckBytes<C>>::check_bytes(ptr, context)
            .map_err(OwnedPointerError::ValueCheckBytesError)?;
        Ok(&*value)
    }
}

impl<
        T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized,
        C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized,
    > CheckBytes<C> for ArchivedBox<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error =
        OwnedPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<T>::manual_check_bytes(value.cast(), context)
            .map_err(OwnedPointerError::PointerCheckBytesError)?;
        let ptr = context
            .claim_owned_rel_ptr(rel_ptr)
            .map_err(OwnedPointerError::ContextError)?;
        T::check_bytes(ptr, context).map_err(OwnedPointerError::ValueCheckBytesError)?;
        Ok(&*value)
    }
}

impl<T: CheckBytes<C>, C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C>
    for ArchivedVec<T>
where
    [T]: ArchivePointee,
    <[T] as ArchivePointee>::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <[T] as Pointee>::Metadata: LayoutMetadata<[T]>,
{
    type Error = CheckOwnedPointerError<[T], C>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<[T]>::manual_check_bytes(value.cast(), context)
            .map_err(OwnedPointerError::PointerCheckBytesError)?;
        let ptr = context
            .claim_owned_rel_ptr(rel_ptr)
            .map_err(OwnedPointerError::ContextError)?;
        <[T]>::check_bytes(ptr, context).map_err(OwnedPointerError::ValueCheckBytesError)?;
        Ok(&*value)
    }
}
