use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8,
};

use crate::{
    Archived,
    niche::{
        niched_option::NichedOption,
        niching::{NaN, Niching, SharedNiching, Zero},
    },
};

macro_rules! impl_nonzero_zero_niching {
    ($nz:ty, $ar:ty) => {
        unsafe impl Niching<Archived<$nz>> for Zero {
            type Niched = Archived<$ar>;

            fn niched_ptr(ptr: *const Archived<$nz>) -> *const Self::Niched {
                ptr.cast()
            }

            fn is_niched(niched: *const Archived<$nz>) -> bool {
                unsafe { *Self::niched_ptr(niched) == 0 }
            }

            fn resolve_niched(out: *mut Archived<$nz>) {
                unsafe { out.cast::<Self::Niched>().write(0.into()) };
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

            fn niched_ptr(ptr: *const Archived<$fl>) -> *const Self::Niched {
                ptr
            }

            fn is_niched(niched: *const Archived<$fl>) -> bool {
                unsafe { (*niched).to_native().is_nan() }
            }

            fn resolve_niched(out: *mut Archived<$fl>) {
                unsafe { out.write(<$fl>::NAN.into()) };
            }
        }
    };
}

impl_float_nan_niching!(f32);
impl_float_nan_niching!(f64);

unsafe impl<T, N1, N2> Niching<NichedOption<T, N1>> for N2
where
    T: Archive<Archived: SharedNiching<N1, N2>>,
    N1: Niching<T::Archived>,
    N2: Niching<T::Archived>,
{
    type Niched = <Self as Niching<T::Archived>>::Niched;

    fn niched_ptr(ptr: *const NichedOption<T, N1>) -> *const Self::Niched {
        <Self as Niching<T::Archived>>::niched_ptr(ptr.cast())
    }

    fn is_niched(niched: *const NichedOption<T, N1>) -> bool {
        <Self as Niching<T::Archived>>::is_niched(niched.cast())
    }

    fn resolve_niched(out: Place<NichedOption<T, N1>>) {
        <Self as Niching<T::Archived>>::resolve_niched(unsafe {
            out.cast_unchecked()
        })
    }
}
