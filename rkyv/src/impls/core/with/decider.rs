use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize,
    NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
};

use crate::{
    niche::decider::{Decider, NaN, Zero},
    Archive, Archived, Place,
};

macro_rules! impl_nonzero_zero_decider {
    ($nz:ty, $ar:ty) => {
        unsafe impl Decider<$nz> for Zero {
            type Niched = Archived<$ar>;

            fn is_niched(niched: &Self::Niched) -> bool {
                *niched == 0
            }

            fn resolve_niche(out: Place<Self::Niched>) {
                <$ar>::resolve(&0, (), out)
            }
        }
    };
}

impl_nonzero_zero_decider!(NonZeroU8, u8);
impl_nonzero_zero_decider!(NonZeroU16, u16);
impl_nonzero_zero_decider!(NonZeroU32, u32);
impl_nonzero_zero_decider!(NonZeroU64, u64);
impl_nonzero_zero_decider!(NonZeroU128, u128);
impl_nonzero_zero_decider!(NonZeroUsize, usize);

impl_nonzero_zero_decider!(NonZeroI8, i8);
impl_nonzero_zero_decider!(NonZeroI16, i16);
impl_nonzero_zero_decider!(NonZeroI32, i32);
impl_nonzero_zero_decider!(NonZeroI64, i64);
impl_nonzero_zero_decider!(NonZeroI128, i128);
impl_nonzero_zero_decider!(NonZeroIsize, isize);

macro_rules! impl_float_nan_decider {
    ($fl:ty) => {
        unsafe impl Decider<$fl> for NaN {
            type Niched = Archived<$fl>;

            fn is_niched(niched: &Self::Niched) -> bool {
                niched.to_native().is_nan()
            }

            fn resolve_niche(out: Place<Self::Niched>) {
                <$fl>::resolve(&<$fl>::NAN, (), out)
            }
        }
    };
}

impl_float_nan_decider!(f32);
impl_float_nan_decider!(f64);
