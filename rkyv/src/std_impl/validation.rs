use core::fmt;
use std::error::Error;
use super::{ArchivedBox, ArchivedString, ArchivedVec};
use crate::{core_impl::{ArchivedStringSlice}, validation::{ArchiveBoundsContext, ArchiveBoundsError, ArchiveMemoryContext, ArchiveMemoryError, CheckBytesRef}};
use bytecheck::CheckBytes;

#[derive(Debug)]
pub enum OwnedPointerError<T, R> {
    BoundsError(ArchiveBoundsError),
    MemoryError(ArchiveMemoryError),
    CheckBytes(T),
    RefCheckBytes(R),
}

impl<T, R> From<ArchiveBoundsError> for OwnedPointerError<T, R> {
    fn from(e: ArchiveBoundsError) -> Self {
        Self::BoundsError(e)
    }
}

impl<T, R> From<ArchiveMemoryError> for OwnedPointerError<T, R> {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl<T: fmt::Display, R: fmt::Display> fmt::Display for OwnedPointerError<T, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OwnedPointerError::BoundsError(e) => write!(f, "{}", e),
            OwnedPointerError::MemoryError(e) => write!(f, "{}", e),
            OwnedPointerError::CheckBytes(e) => write!(f, "{}", e),
            OwnedPointerError::RefCheckBytes(e) => write!(f, "{}", e),
        }
    }
}

impl<T: fmt::Debug + fmt::Display + Error + 'static, R: fmt::Debug + fmt::Display + Error + 'static> Error for OwnedPointerError<T, R> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            OwnedPointerError::BoundsError(e) => Some(e as &dyn Error),
            OwnedPointerError::MemoryError(e) => Some(e as &dyn Error),
            OwnedPointerError::CheckBytes(e) => Some(e as &dyn Error),
            OwnedPointerError::RefCheckBytes(e) => Some(e as &dyn Error),
        }
    }
}

impl<T: CheckBytesRef<C>, C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C> for ArchivedBox<T> {
    type Error = OwnedPointerError<<T as CheckBytes<C>>::Error, <T as CheckBytesRef<C>>::RefError>;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        let reference = T::check_bytes(bytes, context).map_err(OwnedPointerError::CheckBytes)?;
        let (start, len) = reference.check_ptr(context)?;
        let ref_bytes = context.claim_bytes(start, len)?;
        reference.check_ref_bytes(ref_bytes, context).map_err(OwnedPointerError::RefCheckBytes)?;
        Ok(&*bytes.cast())
    }
}

impl<C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C> for ArchivedString
where
    ArchivedStringSlice: CheckBytes<C>,
{
    type Error = OwnedPointerError<<ArchivedStringSlice as CheckBytes<C>>::Error, <ArchivedStringSlice as CheckBytesRef<C>>::RefError>;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        let reference = ArchivedStringSlice::check_bytes(bytes, context).map_err(OwnedPointerError::CheckBytes)?;
        let (start, len) = reference.check_ptr(context)?;
        let ref_bytes = context.claim_bytes(start, len)?;
        reference.check_ref_bytes(ref_bytes, context).map_err(OwnedPointerError::RefCheckBytes)?;
        Ok(&*bytes.cast())
    }
}

impl<T: CheckBytesRef<C>, C: ArchiveBoundsContext + ArchiveMemoryContext + ?Sized> CheckBytes<C> for ArchivedVec<T> {
    type Error = OwnedPointerError<<T as CheckBytes<C>>::Error, <T as CheckBytesRef<C>>::RefError>;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        let reference = T::check_bytes(bytes, context).map_err(OwnedPointerError::CheckBytes)?;
        let (start, len) = reference.check_ptr(context)?;
        let ref_bytes = context.claim_bytes(start, len)?;
        reference.check_ref_bytes(ref_bytes, context).map_err(OwnedPointerError::RefCheckBytes)?;
        Ok(&*bytes.cast())
    }
}
