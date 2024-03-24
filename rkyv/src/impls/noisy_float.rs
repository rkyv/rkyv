use noisy_float::{FloatChecker, NoisyFloat};

use crate::{
    niche::option_float::{ArchivedOptionF32, ArchivedOptionF64},
    with::{ArchiveWith, DeserializeWith, Niche, SerializeWith},
};

use rancor::Fallible;

macro_rules! impl_noisyfloat_niche {
    ($ar:ident, $f:ty) => {
        impl<C: FloatChecker<$f>> ArchiveWith<Option<NoisyFloat<$f, C>>>
            for Niche
        {
            type Archived = $ar<C>;
            type Resolver = ();

            #[inline]
            unsafe fn resolve_with(
                field: &Option<NoisyFloat<$f, C>>,
                _: usize,
                _: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                <$ar<C>>::resolve_from_option(field.map(NoisyFloat::raw), out);
            }
        }

        impl<C: FloatChecker<$f>, S: Fallible + ?Sized>
            SerializeWith<Option<NoisyFloat<$f, C>>, S> for Niche
        {
            #[inline]
            fn serialize_with(
                _: &Option<NoisyFloat<$f, C>>,
                _: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<C: FloatChecker<$f>, D>
            DeserializeWith<$ar<C>, Option<NoisyFloat<$f, C>>, D> for Niche
        where
            D: Fallible + ?Sized,
        {
            #[inline]
            fn deserialize_with(
                field: &$ar<C>,
                _: &mut D,
            ) -> Result<Option<NoisyFloat<$f, C>>, D::Error> {
                Ok(field.as_ref().and_then(|raw| {
                    NoisyFloat::<$f, C>::try_new(raw.to_native())
                }))
            }
        }
    };
}

impl_noisyfloat_niche!(ArchivedOptionF32, f32);
impl_noisyfloat_niche!(ArchivedOptionF64, f64);
