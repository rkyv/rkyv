use crate::core_impl::primitive::{
    I16LE, I16BE, I32LE, I32BE, I64LE, I64BE, I128LE, I128BE, U16LE, U16BE, U32LE, U32BE, U64LE,
    U64BE, U128LE, U128BE, F32LE, F32BE, F64LE, F64BE, CharLE, CharBE, NonZeroI16LE, NonZeroI16BE,
    NonZeroI32LE, NonZeroI32BE, NonZeroI64LE, NonZeroI64BE, NonZeroI128LE, NonZeroI128BE,
    NonZeroU16LE, NonZeroU16BE, NonZeroU32LE, NonZeroU32BE, NonZeroU64LE, NonZeroU64BE,
    NonZeroU128LE, NonZeroU128BE,
};
use core::num::{
    NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI128, NonZeroU16, NonZeroU32, NonZeroU64,
    NonZeroU128,
};
use bytecheck::CheckBytes;

macro_rules! impl_endian {
    ($ne:ty, $le:ty, $be:ty) => {
        impl<C: ?Sized> CheckBytes<C> for $le
        where
            $ne: CheckBytes<C>,
        {
            type Error = <$ne as CheckBytes<C>>::Error;

            #[inline]
            unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
                <$ne>::check_bytes(&<$ne>::from(*value) as *const $ne, context)?;
                Ok(&*value)
            }
        }

        impl<C: ?Sized> CheckBytes<C> for $be
        where
            $ne: CheckBytes<C>,
        {
            type Error = <$ne as CheckBytes<C>>::Error;

            #[inline]
            unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
                <$ne>::check_bytes(&<$ne>::from(*value) as *const $ne, context)?;
                Ok(&*value)
            }
        }
    }
}

impl_endian!(i16, I16LE, I16BE);
impl_endian!(i32, I32LE, I32BE);
impl_endian!(i64, I64LE, I64BE);
impl_endian!(i128, I128LE, I128BE);
impl_endian!(u16, U16LE, U16BE);
impl_endian!(u32, U32LE, U32BE);
impl_endian!(u64, U64LE, U64BE);
impl_endian!(u128, U128LE, U128BE);
impl_endian!(f32, F32LE, F32BE);
impl_endian!(f64, F64LE, F64BE);
impl_endian!(char, CharLE, CharBE);
impl_endian!(NonZeroI16, NonZeroI16LE, NonZeroI16BE);
impl_endian!(NonZeroI32, NonZeroI32LE, NonZeroI32BE);
impl_endian!(NonZeroI64, NonZeroI64LE, NonZeroI64BE);
impl_endian!(NonZeroI128, NonZeroI128LE, NonZeroI128BE);
impl_endian!(NonZeroU16, NonZeroU16LE, NonZeroU16BE);
impl_endian!(NonZeroU32, NonZeroU32LE, NonZeroU32BE);
impl_endian!(NonZeroU64, NonZeroU64LE, NonZeroU64BE);
impl_endian!(NonZeroU128, NonZeroU128LE, NonZeroU128BE);
