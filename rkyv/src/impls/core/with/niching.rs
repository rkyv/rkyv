use core::num::{NonZeroI8, NonZeroU8};

use crate::{
    niche::{
        niched_option::NichedOption,
        niching::{DefaultNicher, NaN, Niching, SharedNiching, Zero},
    },
    primitive::{
        ArchivedF32, ArchivedF64, ArchivedNonZeroI128, ArchivedNonZeroI16,
        ArchivedNonZeroI32, ArchivedNonZeroI64, ArchivedNonZeroU128,
        ArchivedNonZeroU16, ArchivedNonZeroU32, ArchivedNonZeroU64,
    },
    Archive, Archived, Place,
};

// Zero

macro_rules! impl_nonzero_zero_niching {
    ($nz:ty, $ar:ty) => {
        unsafe impl Niching<$nz> for Zero {
            type Niched = Archived<$ar>;

            fn niched_ptr(ptr: *const $nz) -> *const Self::Niched {
                ptr.cast()
            }

            unsafe fn is_niched(niched: *const $nz) -> bool {
                unsafe { *Self::niched_ptr(niched) == 0 }
            }

            fn resolve_niched(out: Place<$nz>) {
                unsafe { out.cast_unchecked::<Self::Niched>() }.write(0.into());
            }
        }

        unsafe impl Niching<$nz> for DefaultNicher {
            type Niched = <Zero as Niching<$nz>>::Niched;

            fn niched_ptr(ptr: *const $nz) -> *const Self::Niched {
                <Zero as Niching<$nz>>::niched_ptr(ptr)
            }

            unsafe fn is_niched(niched: *const $nz) -> bool {
                unsafe { <Zero as Niching<$nz>>::is_niched(niched) }
            }

            fn resolve_niched(out: Place<$nz>) {
                <Zero as Niching<$nz>>::resolve_niched(out);
            }
        }
    };
}

impl_nonzero_zero_niching!(NonZeroU8, u8);
impl_nonzero_zero_niching!(ArchivedNonZeroU16, u16);
impl_nonzero_zero_niching!(ArchivedNonZeroU32, u32);
impl_nonzero_zero_niching!(ArchivedNonZeroU64, u64);
impl_nonzero_zero_niching!(ArchivedNonZeroU128, u128);

impl_nonzero_zero_niching!(NonZeroI8, i8);
impl_nonzero_zero_niching!(ArchivedNonZeroI16, i16);
impl_nonzero_zero_niching!(ArchivedNonZeroI32, i32);
impl_nonzero_zero_niching!(ArchivedNonZeroI64, i64);
impl_nonzero_zero_niching!(ArchivedNonZeroI128, i128);

// NaN

macro_rules! impl_float_nan_niching {
    ($fl:ty, $ar:ty) => {
        unsafe impl Niching<$ar> for NaN {
            type Niched = $ar;

            fn niched_ptr(ptr: *const $ar) -> *const Self::Niched {
                ptr
            }

            unsafe fn is_niched(niched: *const $ar) -> bool {
                unsafe { (*niched).to_native().is_nan() }
            }

            fn resolve_niched(out: Place<$ar>) {
                out.write(<$fl>::NAN.into());
            }
        }
    };
}

impl_float_nan_niching!(f32, ArchivedF32);
impl_float_nan_niching!(f64, ArchivedF64);

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

    unsafe fn is_niched(niched: *const NichedOption<T, N1>) -> bool {
        unsafe { <Self as Niching<T::Archived>>::is_niched(niched.cast()) }
    }

    fn resolve_niched(out: Place<NichedOption<T, N1>>) {
        <Self as Niching<T::Archived>>::resolve_niched(unsafe {
            out.cast_unchecked()
        })
    }
}
