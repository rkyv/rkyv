use crate::{
    boxed::{ArchivedBox, BoxResolver},
    niche::option_nonzero::{
        ArchivedOptionNonZeroI128, ArchivedOptionNonZeroI16, ArchivedOptionNonZeroI32,
        ArchivedOptionNonZeroI64, ArchivedOptionNonZeroI8, ArchivedOptionNonZeroU128,
        ArchivedOptionNonZeroU16, ArchivedOptionNonZeroU32, ArchivedOptionNonZeroU64,
        ArchivedOptionNonZeroU8,
    },
    with::{ArchiveWith, AsBox, DeserializeWith, Inline, Niche, RefAsBox, SerializeWith, Unsafe},
    Archive, ArchiveUnsized, Deserialize, Fallible, Serialize, SerializeUnsized,
};
use ::core::{
    cell::{Cell, UnsafeCell},
    convert::TryInto,
    num::{
        NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroIsize, NonZeroU128,
        NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8, NonZeroUsize,
    },
};

// Inline

impl<F: Archive> ArchiveWith<&F> for Inline {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    #[inline]
    unsafe fn resolve_with(
        field: &&F,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        field.resolve(pos, resolver, out);
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<&F, S> for Inline {
    #[inline]
    fn serialize_with(field: &&F, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.serialize(serializer)
    }
}

// RefAsBox

impl<F: ArchiveUnsized + ?Sized> ArchiveWith<&F> for RefAsBox {
    type Archived = ArchivedBox<F::Archived>;
    type Resolver = BoxResolver<F::MetadataResolver>;

    #[inline]
    unsafe fn resolve_with(
        field: &&F,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedBox::resolve_from_ref(*field, pos, resolver, out);
    }
}

impl<F: SerializeUnsized<S> + ?Sized, S: Fallible + ?Sized> SerializeWith<&F, S> for RefAsBox {
    #[inline]
    fn serialize_with(field: &&F, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedBox::serialize_from_ref(*field, serializer)
    }
}

// AsBox

impl<F: ArchiveUnsized + ?Sized> ArchiveWith<F> for AsBox {
    type Archived = ArchivedBox<F::Archived>;
    type Resolver = BoxResolver<F::MetadataResolver>;

    #[inline]
    unsafe fn resolve_with(
        field: &F,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedBox::resolve_from_ref(field, pos, resolver, out);
    }
}

impl<F: SerializeUnsized<S> + ?Sized, S: Fallible + ?Sized> SerializeWith<F, S> for AsBox {
    #[inline]
    fn serialize_with(field: &F, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedBox::serialize_from_ref(field, serializer)
    }
}

impl<F: Archive, D: Fallible + ?Sized> DeserializeWith<F::Archived, F, D> for AsBox
where
    F::Archived: Deserialize<F, D>,
{
    #[inline]
    fn deserialize_with(field: &F::Archived, deserializer: &mut D) -> Result<F, D::Error> {
        field.deserialize(deserializer)
    }
}

// Niche

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
            fn serialize_with(_: &Option<$nz>, _: &mut S) -> Result<Self::Resolver, S::Error> {
                Ok(())
            }
        }

        impl<D: Fallible + ?Sized> DeserializeWith<$ar, Option<$nz>, D> for Niche {
            #[inline]
            fn deserialize_with(field: &$ar, _: &mut D) -> Result<Option<$nz>, D::Error> {
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

type FixedNonZeroIsize = pick_size_type!(NonZeroI16, NonZeroI32, NonZeroI64);
type ArchivedOptionNonZeroIsize = pick_size_type!(
    ArchivedOptionNonZeroI16,
    ArchivedOptionNonZeroI32,
    ArchivedOptionNonZeroI64,
);

impl ArchiveWith<Option<NonZeroIsize>> for Niche {
    type Archived = ArchivedOptionNonZeroIsize;
    type Resolver = ();

    #[inline]
    unsafe fn resolve_with(
        field: &Option<NonZeroIsize>,
        _: usize,
        _: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let f = field.as_ref().map(|&x| x.try_into().unwrap());
        ArchivedOptionNonZeroIsize::resolve_from_option(f, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Option<NonZeroIsize>, S> for Niche {
    #[inline]
    fn serialize_with(_: &Option<NonZeroIsize>, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedOptionNonZeroIsize, Option<NonZeroIsize>, D>
    for Niche
{
    #[inline]
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

impl_nonzero_niche!(ArchivedOptionNonZeroU8, NonZeroU8, u8);
impl_nonzero_niche!(ArchivedOptionNonZeroU16, NonZeroU16, u16);
impl_nonzero_niche!(ArchivedOptionNonZeroU32, NonZeroU32, u32);
impl_nonzero_niche!(ArchivedOptionNonZeroU64, NonZeroU64, u64);
impl_nonzero_niche!(ArchivedOptionNonZeroU128, NonZeroU128, u128);

type FixedNonZeroUsize = pick_size_type!(NonZeroU16, NonZeroU32, NonZeroU64);
type ArchivedOptionNonZeroUsize = pick_size_type!(
    ArchivedOptionNonZeroU16,
    ArchivedOptionNonZeroU32,
    ArchivedOptionNonZeroU64,
);

impl ArchiveWith<Option<NonZeroUsize>> for Niche {
    type Archived = ArchivedOptionNonZeroUsize;
    type Resolver = ();

    #[inline]
    unsafe fn resolve_with(
        field: &Option<NonZeroUsize>,
        _: usize,
        _: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let f = field.as_ref().map(|&x| x.try_into().unwrap());
        ArchivedOptionNonZeroUsize::resolve_from_option(f, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<Option<NonZeroUsize>, S> for Niche {
    #[inline]
    fn serialize_with(_: &Option<NonZeroUsize>, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedOptionNonZeroUsize, Option<NonZeroUsize>, D>
    for Niche
{
    #[inline]
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

// Unsafe

impl<F: Archive> ArchiveWith<UnsafeCell<F>> for Unsafe {
    type Archived = UnsafeCell<F::Archived>;
    type Resolver = F::Resolver;

    #[inline]
    unsafe fn resolve_with(
        field: &UnsafeCell<F>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        F::resolve(&*field.get(), pos, resolver, out.cast());
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<UnsafeCell<F>, S> for Unsafe {
    #[inline]
    fn serialize_with(
        field: &UnsafeCell<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        unsafe { (&*field.get()).serialize(serializer) }
    }
}

impl<F: Archive, D: Fallible + ?Sized> DeserializeWith<UnsafeCell<F::Archived>, UnsafeCell<F>, D>
    for Unsafe
where
    F::Archived: Deserialize<F, D>,
{
    #[inline]
    fn deserialize_with(
        field: &UnsafeCell<F::Archived>,
        deserializer: &mut D,
    ) -> Result<UnsafeCell<F>, D::Error> {
        unsafe {
            (&*field.get())
                .deserialize(deserializer)
                .map(|x| UnsafeCell::new(x))
        }
    }
}

impl<F: Archive> ArchiveWith<Cell<F>> for Unsafe {
    type Archived = Cell<F::Archived>;
    type Resolver = F::Resolver;

    #[inline]
    unsafe fn resolve_with(
        field: &Cell<F>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        F::resolve(&*field.as_ptr(), pos, resolver, out.cast());
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<Cell<F>, S> for Unsafe {
    #[inline]
    fn serialize_with(field: &Cell<F>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        unsafe { (&*field.as_ptr()).serialize(serializer) }
    }
}

impl<F: Archive, D: Fallible + ?Sized> DeserializeWith<Cell<F::Archived>, Cell<F>, D> for Unsafe
where
    F::Archived: Deserialize<F, D>,
{
    #[inline]
    fn deserialize_with(
        field: &Cell<F::Archived>,
        deserializer: &mut D,
    ) -> Result<Cell<F>, D::Error> {
        unsafe {
            (&*field.as_ptr())
                .deserialize(deserializer)
                .map(|x| Cell::new(x))
        }
    }
}
