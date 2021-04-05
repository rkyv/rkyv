//! Validation implementations for shared pointers.

use super::{
    ArchivedArc, ArchivedArcWeak, ArchivedArcWeakTag, ArchivedArcWeakVariantSome, ArchivedRc,
    ArchivedRcWeak, ArchivedRcWeakTag, ArchivedRcWeakVariantSome,
};
use crate::{
    offset_of,
    validation::{ArchiveBoundsContext, LayoutMetadata, SharedArchiveContext},
    ArchivePointee, RelPtr,
};
use bytecheck::{CheckBytes, Unreachable};
use core::{any::TypeId, fmt};
use ptr_meta::Pointee;
use std::error::Error;

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

impl<T: fmt::Display, R: fmt::Display, C: fmt::Display> fmt::Display
    for SharedPointerError<T, R, C>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedPointerError::PointerCheckBytesError(e) => e.fmt(f),
            SharedPointerError::ValueCheckBytesError(e) => e.fmt(f),
            SharedPointerError::ContextError(e) => e.fmt(f),
        }
    }
}

impl<T: Error + 'static, R: Error + 'static, C: Error + 'static> Error
    for SharedPointerError<T, R, C>
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedPointerError::PointerCheckBytesError(e) => Some(e as &dyn Error),
            SharedPointerError::ValueCheckBytesError(e) => Some(e as &dyn Error),
            SharedPointerError::ContextError(e) => Some(e as &dyn Error),
        }
    }
}

/// Errors that can occur while checking archived weak pointers.
#[derive(Debug)]
pub enum WeakPointerError<T, R, C> {
    /// The weak pointer had an invalid tag
    InvalidTag(u8),
    /// An error occurred while checking the underlying shared pointer
    CheckBytes(SharedPointerError<T, R, C>),
}

impl<T: fmt::Display, R: fmt::Display, C: fmt::Display> fmt::Display for WeakPointerError<T, R, C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WeakPointerError::InvalidTag(tag) => {
                write!(f, "archived weak had invalid tag: {}", tag)
            }
            WeakPointerError::CheckBytes(e) => e.fmt(f),
        }
    }
}

impl<T: Error + 'static, R: Error + 'static, C: Error + 'static> Error
    for WeakPointerError<T, R, C>
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            WeakPointerError::InvalidTag(_) => None,
            WeakPointerError::CheckBytes(e) => Some(e as &dyn Error),
        }
    }
}

impl<T, R, C> From<Unreachable> for WeakPointerError<T, R, C> {
    fn from(_: Unreachable) -> Self {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

impl<
        T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized + 'static,
        C: ArchiveBoundsContext + SharedArchiveContext + ?Sized,
    > CheckBytes<C> for ArchivedRc<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error =
        SharedPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<T>::manual_check_bytes(value.cast(), context)
            .map_err(SharedPointerError::PointerCheckBytesError)?;
        if let Some(ptr) = context
            .claim_shared_ptr(rel_ptr, TypeId::of::<ArchivedRc<T>>())
            .map_err(SharedPointerError::ContextError)?
        {
            T::check_bytes(ptr, context).map_err(SharedPointerError::ValueCheckBytesError)?;
        }
        Ok(&*value)
    }
}

impl ArchivedRcWeakTag {
    const TAG_NONE: u8 = ArchivedRcWeakTag::None as u8;
    const TAG_SOME: u8 = ArchivedRcWeakTag::Some as u8;
}

impl<
        T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized + 'static,
        C: ArchiveBoundsContext + SharedArchiveContext + ?Sized,
    > CheckBytes<C> for ArchivedRcWeak<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error =
        WeakPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        let tag = *u8::check_bytes(bytes, context)?;
        match tag {
            ArchivedRcWeakTag::TAG_NONE => (),
            ArchivedRcWeakTag::TAG_SOME => {
                ArchivedRc::<T>::check_bytes(
                    bytes
                        .add(offset_of!(ArchivedRcWeakVariantSome<T>, 1))
                        .cast(),
                    context,
                )
                .map_err(WeakPointerError::CheckBytes)?;
            }
            _ => return Err(WeakPointerError::InvalidTag(tag)),
        }
        Ok(&*value)
    }
}

impl<
        T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized + 'static,
        C: ArchiveBoundsContext + SharedArchiveContext + ?Sized,
    > CheckBytes<C> for ArchivedArc<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error =
        SharedPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::<T>::manual_check_bytes(value.cast(), context)
            .map_err(SharedPointerError::PointerCheckBytesError)?;
        if let Some(ptr) = context
            .claim_shared_ptr(rel_ptr, TypeId::of::<ArchivedArc<T>>())
            .map_err(SharedPointerError::ContextError)?
        {
            T::check_bytes(ptr, context).map_err(SharedPointerError::ValueCheckBytesError)?;
        }
        Ok(&*value)
    }
}

impl ArchivedArcWeakTag {
    const TAG_NONE: u8 = ArchivedArcWeakTag::None as u8;
    const TAG_SOME: u8 = ArchivedArcWeakTag::Some as u8;
}

impl<
        T: ArchivePointee + CheckBytes<C> + Pointee + ?Sized + 'static,
        C: ArchiveBoundsContext + SharedArchiveContext + ?Sized,
    > CheckBytes<C> for ArchivedArcWeak<T>
where
    T::ArchivedMetadata: CheckBytes<C>,
    C::Error: Error,
    <T as Pointee>::Metadata: LayoutMetadata<T>,
{
    type Error =
        WeakPointerError<<T::ArchivedMetadata as CheckBytes<C>>::Error, T::Error, C::Error>;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        let tag = *u8::check_bytes(bytes, context)?;
        match tag {
            ArchivedArcWeakTag::TAG_NONE => (),
            ArchivedArcWeakTag::TAG_SOME => {
                ArchivedArc::<T>::check_bytes(
                    bytes
                        .add(offset_of!(ArchivedArcWeakVariantSome<T>, 1))
                        .cast(),
                    context,
                )
                .map_err(WeakPointerError::CheckBytes)?;
            }
            _ => return Err(WeakPointerError::InvalidTag(tag)),
        }
        Ok(&*value)
    }
}
