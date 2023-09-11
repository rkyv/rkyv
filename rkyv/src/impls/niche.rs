use core::num::{
    NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU128,
    NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8,
};

use crate::{
    niche::option_nonzero::{
        ArchivedOptionNonZeroI128, ArchivedOptionNonZeroI16,
        ArchivedOptionNonZeroI32, ArchivedOptionNonZeroI64,
        ArchivedOptionNonZeroI8, ArchivedOptionNonZeroU128,
        ArchivedOptionNonZeroU16, ArchivedOptionNonZeroU32,
        ArchivedOptionNonZeroU64, ArchivedOptionNonZeroU8,
    },
    with::{ArchiveWith, DeserializeWith, Niche, SerializeWith},
    Fallible,
};

macro_rules! impl_nonzero_niche {
    ($ar:ty, $nz:ty, $ne:ty) => {
        impl ArchiveWith<Option<$nz>> for Niche {
            type Archived = $ar;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &Option<$nz>,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                <$ar>::resolve_from_option(*field, out);
            }
        }

        impl<S: Fallible + ?Sized> SerializeWith<Option<$nz>, S> for Niche {
            #[inline]
            fn serialize_with(
                _: &Option<$nz>,
                _: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> DeserializeWith<$ar, Option<$nz>, D>
            for Niche
        {
            #[inline]
            fn deserialize_with(
                field: &$ar,
                _: &mut D,
            ) -> Result<Option<$nz>, D::Error> {
                Ok(field.as_ref().map(|x| (*x).into()))
            }
        }
    };
}

impl_nonzero_niche!(ArchivedOptionNonZeroI8, NonZeroI8, i8);
impl_nonzero_niche!(ArchivedOptionNonZeroI16, NonZeroI16, i16);
impl_nonzero_niche!(ArchivedOptionNonZeroI32, NonZeroI32, i32);
impl_nonzero_niche!(ArchivedOptionNonZeroI64, NonZeroI64, i64);
impl_nonzero_niche!(ArchivedOptionNonZeroI128, NonZeroI128, i128);

impl_nonzero_niche!(ArchivedOptionNonZeroU8, NonZeroU8, u8);
impl_nonzero_niche!(ArchivedOptionNonZeroU16, NonZeroU16, u16);
impl_nonzero_niche!(ArchivedOptionNonZeroU32, NonZeroU32, u32);
impl_nonzero_niche!(ArchivedOptionNonZeroU64, NonZeroU64, u64);
impl_nonzero_niche!(ArchivedOptionNonZeroU128, NonZeroU128, u128);
