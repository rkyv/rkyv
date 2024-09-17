use core::{
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
    ptr,
};

use crate::{
    niche::decider::{Decider, NaN, Zero},
    Archive, Archived, Place, Resolver,
};

macro_rules! impl_nonzero_zero_decider {
    ($nz:ty, $ar:ty) => {
        impl Decider<$nz> for Zero {
            type Archived = Archived<$ar>;

            fn as_option(archived: &Self::Archived) -> Option<&Archived<$nz>> {
                if *archived == 0 {
                    None
                } else {
                    // SAFETY: NonZero types have the same memory layout and
                    // bit patterns as their integer counterparts,
                    // regardless of endianness.
                    let as_nonzero = unsafe {
                        &*(ptr::from_ref(archived).cast::<Archived<$nz>>())
                    };

                    Some(as_nonzero)
                }
            }

            fn resolve_from_option(
                option: Option<&$nz>,
                resolver: Option<Resolver<$nz>>,
                out: Place<Self::Archived>,
            ) {
                match option {
                    Some(value) => {
                        let resolver = resolver.expect("non-niched resolver");
                        value.get().resolve(resolver, out);
                    }
                    None => <$ar>::resolve(&0, (), out),
                }
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
        impl Decider<$fl> for NaN {
            type Archived = Archived<$fl>;

            fn as_option(archived: &Self::Archived) -> Option<&Archived<$fl>> {
                if archived.to_native().is_nan() {
                    None
                } else {
                    Some(archived)
                }
            }

            fn resolve_from_option(
                option: Option<&$fl>,
                resolver: Option<Resolver<$fl>>,
                out: Place<Self::Archived>,
            ) {
                match option {
                    Some(value) => {
                        let resolver = resolver.expect("non-niched resolver");
                        value.resolve(resolver, out);
                    }
                    None => <$fl>::resolve(&<$fl>::NAN, (), out),
                }
            }
        }
    };
}

impl_float_nan_decider!(f32);
impl_float_nan_decider!(f64);
