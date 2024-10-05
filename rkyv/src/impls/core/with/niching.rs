use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8,
};

use crate::{
    niche::niching::{NaN, Niching, Zero},
    Archived,
};

macro_rules! impl_nonzero_zero_niching {
    ($nz:ty, $ar:ty) => {
        unsafe impl Niching<Archived<$nz>> for Zero {
            type Niched = Archived<$ar>;

            fn is_niched(niched: *const Archived<$nz>) -> bool {
                unsafe { *niched.cast::<Self::Niched>() == 0 }
            }

            fn resolve_niched(out: *mut Archived<$nz>) {
                unsafe { out.cast::<Self::Niched>().write(0.into()) };
            }

            #[cfg(feature = "bytecheck")]
            unsafe fn checked_is_niched<C>(
                niched: *const Archived<$nz>,
                context: &mut C,
            ) -> Result<bool, C::Error>
            where
                C: rancor::Fallible + ?Sized,
                Self::Niched: bytecheck::CheckBytes<C>,
            {
                unsafe {
                    <Archived<$ar> as bytecheck::CheckBytes<C>>::check_bytes(
                        niched.cast::<Self::Niched>(),
                        context,
                    )?
                };

                Ok(Self::is_niched(niched))
            }
        }
    };
}

impl_nonzero_zero_niching!(NonZeroU8, u8);
impl_nonzero_zero_niching!(NonZeroU16, u16);
impl_nonzero_zero_niching!(NonZeroU32, u32);
impl_nonzero_zero_niching!(NonZeroU64, u64);
impl_nonzero_zero_niching!(NonZeroU128, u128);

impl_nonzero_zero_niching!(NonZeroI8, i8);
impl_nonzero_zero_niching!(NonZeroI16, i16);
impl_nonzero_zero_niching!(NonZeroI32, i32);
impl_nonzero_zero_niching!(NonZeroI64, i64);
impl_nonzero_zero_niching!(NonZeroI128, i128);

macro_rules! impl_float_nan_niching {
    ($fl:ty) => {
        unsafe impl Niching<Archived<$fl>> for NaN {
            type Niched = Archived<$fl>;

            fn is_niched(niched: *const Archived<$fl>) -> bool {
                unsafe { *niched }.to_native().is_nan()
            }

            fn resolve_niched(out: *mut Archived<$fl>) {
                unsafe { out.write(<$fl>::NAN.into()) };
            }

            #[cfg(feature = "bytecheck")]
            unsafe fn checked_is_niched<C>(
                niched: *const Archived<$fl>,
                context: &mut C,
            ) -> Result<bool, C::Error>
            where
                C: rancor::Fallible + ?Sized,
                Self::Niched: bytecheck::CheckBytes<C>,
            {
                unsafe {
                    <Archived<$fl> as bytecheck::CheckBytes<C>>::check_bytes(
                        niched.cast::<Self::Niched>(),
                        context,
                    )?
                };

                Ok(Self::is_niched(niched))
            }
        }
    };
}

impl_float_nan_niching!(f32);
impl_float_nan_niching!(f64);
