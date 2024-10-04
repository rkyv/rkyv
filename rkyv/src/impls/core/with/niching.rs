use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
    NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

use crate::{
    niche::niching::{NaN, Niching, Zero},
    Archived,
};

macro_rules! impl_nonzero_zero_niching {
    ($nz:ty, $ar:ty) => {
        unsafe impl Niching<$nz> for Zero {
            type Niched = $ar;

            fn niched() -> Self::Niched {
                0
            }

            fn is_niched(niched: &Archived<Self::Niched>) -> bool {
                *niched == 0
            }
        }
    };
}

impl_nonzero_zero_niching!(NonZeroU8, u8);
impl_nonzero_zero_niching!(NonZeroU16, u16);
impl_nonzero_zero_niching!(NonZeroU32, u32);
impl_nonzero_zero_niching!(NonZeroU64, u64);
impl_nonzero_zero_niching!(NonZeroU128, u128);
impl_nonzero_zero_niching!(NonZeroUsize, usize);

impl_nonzero_zero_niching!(NonZeroI8, i8);
impl_nonzero_zero_niching!(NonZeroI16, i16);
impl_nonzero_zero_niching!(NonZeroI32, i32);
impl_nonzero_zero_niching!(NonZeroI64, i64);
impl_nonzero_zero_niching!(NonZeroI128, i128);
impl_nonzero_zero_niching!(NonZeroIsize, isize);

macro_rules! impl_float_nan_niching {
    ($fl:ty) => {
        unsafe impl Niching<$fl> for NaN {
            type Niched = $fl;

            fn niched() -> Self::Niched {
                <$fl>::NAN
            }

            fn is_niched(niched: &Archived<Self::Niched>) -> bool {
                niched.to_native().is_nan()
            }
        }
    };
}

impl_float_nan_niching!(f32);
impl_float_nan_niching!(f64);
