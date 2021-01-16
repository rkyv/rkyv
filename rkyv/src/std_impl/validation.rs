use super::{ArchivedBox, ArchivedString, ArchivedVec};
use crate::core_impl::ArchivedStringSlice;
use bytecheck::CheckBytes;

impl<T: CheckBytes<C>, C: ?Sized> CheckBytes<C> for ArchivedBox<T> {
    type Error = <T as CheckBytes<C>>::Error;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        T::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}

impl<C: ?Sized> CheckBytes<C> for ArchivedString
where
    ArchivedStringSlice: CheckBytes<C>,
{
    type Error = <ArchivedStringSlice as CheckBytes<C>>::Error;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        ArchivedStringSlice::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}

impl<T: CheckBytes<C>, C: ?Sized> CheckBytes<C> for ArchivedVec<T> {
    type Error = <T as CheckBytes<C>>::Error;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        T::check_bytes(bytes, context)?;
        Ok(&*bytes.cast())
    }
}
