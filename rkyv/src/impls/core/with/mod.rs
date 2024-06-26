mod atomic;

use core::{
    cell::{Cell, UnsafeCell},
    hint::unreachable_unchecked,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
        NonZeroIsize, NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64,
        NonZeroU8, NonZeroUsize,
    },
};

use munge::munge;
use rancor::Fallible;

use crate::{
    niche::option_nonzero::{
        ArchivedOptionNonZeroI128, ArchivedOptionNonZeroI16,
        ArchivedOptionNonZeroI32, ArchivedOptionNonZeroI64,
        ArchivedOptionNonZeroI8, ArchivedOptionNonZeroIsize,
        ArchivedOptionNonZeroU128, ArchivedOptionNonZeroU16,
        ArchivedOptionNonZeroU32, ArchivedOptionNonZeroU64,
        ArchivedOptionNonZeroU8, ArchivedOptionNonZeroUsize,
    },
    option::ArchivedOption,
    place::Initialized,
    primitive::{FixedNonZeroIsize, FixedNonZeroUsize},
    with::{
        ArchiveWith, DeserializeWith, Inline, Map, Niche, SerializeWith, Skip,
        Unsafe,
    },
    Archive, Deserialize, Place, Serialize,
};

// Map

// Copy-paste from Option's impls for the most part
impl<A, O> ArchiveWith<Option<O>> for Map<A>
where
    A: ArchiveWith<O>,
{
    type Archived = ArchivedOption<<A as ArchiveWith<O>>::Archived>;
    type Resolver = Option<<A as ArchiveWith<O>>::Resolver>;

    fn resolve_with(
        field: &Option<O>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        match resolver {
            None => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedOptionVariantNone>()
                };
                munge!(let ArchivedOptionVariantNone(tag) = out);
                tag.write(ArchivedOptionTag::None);
            }
            Some(resolver) => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedOptionVariantSome<
                        <A as ArchiveWith<O>>::Archived,
                    >>()
                };
                munge!(let ArchivedOptionVariantSome(tag, out_value) = out);
                tag.write(ArchivedOptionTag::Some);

                let value = if let Some(value) = field.as_ref() {
                    value
                } else {
                    unsafe {
                        unreachable_unchecked();
                    }
                };

                A::resolve_with(value, resolver, out_value);
            }
        }
    }
}

impl<A, O, S> SerializeWith<Option<O>, S> for Map<A>
where
    S: Fallible + ?Sized,
    A: ArchiveWith<O> + SerializeWith<O, S>,
{
    fn serialize_with(
        field: &Option<O>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field
            .as_ref()
            .map(|value| A::serialize_with(value, s))
            .transpose()
    }
}

impl<A, O, D>
    DeserializeWith<
        ArchivedOption<<A as ArchiveWith<O>>::Archived>,
        Option<O>,
        D,
    > for Map<A>
where
    D: Fallible + ?Sized,
    A: ArchiveWith<O> + DeserializeWith<<A as ArchiveWith<O>>::Archived, O, D>,
{
    fn deserialize_with(
        field: &ArchivedOption<<A as ArchiveWith<O>>::Archived>,
        d: &mut D,
    ) -> Result<Option<O>, D::Error> {
        match field {
            ArchivedOption::Some(value) => {
                Ok(Some(A::deserialize_with(value, d)?))
            }
            ArchivedOption::None => Ok(None),
        }
    }
}

#[repr(u8)]
enum ArchivedOptionTag {
    None,
    Some,
}

// SAFETY: `ArchivedOptionTag` is `repr(u8)` and so is always initialized.
unsafe impl Initialized for ArchivedOptionTag {}

#[repr(C)]
struct ArchivedOptionVariantNone(ArchivedOptionTag);

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

// Niche

macro_rules! impl_nonzero_niche {
    ($ar:ty, $nz:ty, $ne:ty) => {
        impl ArchiveWith<Option<$nz>> for Niche {
            type Archived = $ar;
            type Resolver = ();

            #[inline]
            fn resolve_with(
                field: &Option<$nz>,
                _: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                <$ar>::resolve_from_option(*field, out);
            }
        }

        impl<S: Fallible + ?Sized> SerializeWith<Option<$nz>, S> for Niche {
            fn serialize_with(
                _: &Option<$nz>,
                _: &mut S,
            ) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D> DeserializeWith<$ar, Option<$nz>, D> for Niche
        where
            D: Fallible + ?Sized,
        {
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

impl ArchiveWith<Option<NonZeroIsize>> for Niche {
    type Archived = ArchivedOptionNonZeroIsize;
    type Resolver = ();

    #[inline]
    fn resolve_with(
        field: &Option<NonZeroIsize>,
        _: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let f = field.as_ref().map(|&x| x.try_into().unwrap());
        ArchivedOptionNonZeroIsize::resolve_from_option(f, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Option<NonZeroIsize>, S> for Niche {
    fn serialize_with(
        _: &Option<NonZeroIsize>,
        _: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D> DeserializeWith<ArchivedOptionNonZeroIsize, Option<NonZeroIsize>, D>
    for Niche
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedOptionNonZeroIsize,
        _: &mut D,
    ) -> Result<Option<NonZeroIsize>, D::Error> {
        // This conversion is necessary with archive_be and archive_le
        #[allow(clippy::useless_conversion)]
        Ok(field
            .as_ref()
            .map(|x| FixedNonZeroIsize::from(*x).try_into().unwrap()))
    }
}

impl ArchiveWith<Option<NonZeroUsize>> for Niche {
    type Archived = ArchivedOptionNonZeroUsize;
    type Resolver = ();

    #[inline]
    fn resolve_with(
        field: &Option<NonZeroUsize>,
        _: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let f = field.as_ref().map(|&x| x.try_into().unwrap());
        ArchivedOptionNonZeroUsize::resolve_from_option(f, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Option<NonZeroUsize>, S> for Niche {
    fn serialize_with(
        _: &Option<NonZeroUsize>,
        _: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D> DeserializeWith<ArchivedOptionNonZeroUsize, Option<NonZeroUsize>, D>
    for Niche
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedOptionNonZeroUsize,
        _: &mut D,
    ) -> Result<Option<NonZeroUsize>, D::Error> {
        // This conversion is necessary with archive_be and archive_le
        #[allow(clippy::useless_conversion)]
        Ok(field
            .as_ref()
            .map(|x| FixedNonZeroUsize::from(*x).try_into().unwrap()))
    }
}

// Inline

impl<F: Archive> ArchiveWith<&F> for Inline {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &&F,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        field.resolve(resolver, out);
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<&F, S> for Inline {
    fn serialize_with(
        field: &&F,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field.serialize(serializer)
    }
}

// Unsafe

impl<F: Archive> ArchiveWith<UnsafeCell<F>> for Unsafe {
    type Archived = UnsafeCell<F::Archived>;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &UnsafeCell<F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let value = unsafe { &*field.get() };
        let out = unsafe { out.cast_unchecked() };
        F::resolve(value, resolver, out);
    }
}

impl<F, S> SerializeWith<UnsafeCell<F>, S> for Unsafe
where
    F: Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize_with(
        field: &UnsafeCell<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        unsafe { (*field.get()).serialize(serializer) }
    }
}

impl<F, D> DeserializeWith<UnsafeCell<F::Archived>, UnsafeCell<F>, D> for Unsafe
where
    F: Archive,
    F::Archived: Deserialize<F, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &UnsafeCell<F::Archived>,
        deserializer: &mut D,
    ) -> Result<UnsafeCell<F>, D::Error> {
        unsafe {
            (*field.get())
                .deserialize(deserializer)
                .map(|x| UnsafeCell::new(x))
        }
    }
}

impl<F: Archive> ArchiveWith<Cell<F>> for Unsafe {
    type Archived = Cell<F::Archived>;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &Cell<F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let value = unsafe { &*field.as_ptr() };
        let out = unsafe { out.cast_unchecked() };
        F::resolve(value, resolver, out);
    }
}

impl<F, S> SerializeWith<Cell<F>, S> for Unsafe
where
    F: Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize_with(
        field: &Cell<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        unsafe { (*field.as_ptr()).serialize(serializer) }
    }
}

impl<F, D> DeserializeWith<Cell<F::Archived>, Cell<F>, D> for Unsafe
where
    F: Archive,
    F::Archived: Deserialize<F, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &Cell<F::Archived>,
        deserializer: &mut D,
    ) -> Result<Cell<F>, D::Error> {
        unsafe {
            (*field.as_ptr())
                .deserialize(deserializer)
                .map(|x| Cell::new(x))
        }
    }
}

// Skip

impl<F> ArchiveWith<F> for Skip {
    type Archived = ();
    type Resolver = ();

    fn resolve_with(_: &F, _: Self::Resolver, _: Place<Self::Archived>) {}
}

impl<F, S: Fallible + ?Sized> SerializeWith<F, S> for Skip {
    fn serialize_with(_: &F, _: &mut S) -> Result<(), S::Error> {
        Ok(())
    }
}

impl<F: Default, D: Fallible + ?Sized> DeserializeWith<(), F, D> for Skip {
    fn deserialize_with(_: &(), _: &mut D) -> Result<F, D::Error> {
        Ok(Default::default())
    }
}
