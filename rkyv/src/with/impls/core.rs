use crate::{
    boxed::{ArchivedBox, BoxResolver},
    niche::option_nonzero::{
        ArchivedOptionNonZeroIsize, ArchivedOptionNonZeroUsize,
    },
    option::ArchivedOption,
    primitive::{FixedNonZeroIsize, FixedNonZeroUsize},
    with::{
        ArchiveWith, Boxed, BoxedInline, DeserializeWith, Inline, Map, Niche,
        SerializeWith, Skip, Unsafe,
    },
    Archive, ArchiveUnsized, Deserialize, Serialize,
    SerializeUnsized,
};
use core::{
    cell::{Cell, UnsafeCell},
    convert::TryInto,
    hint::unreachable_unchecked,
    num::{NonZeroIsize, NonZeroUsize},
    ptr,
};

// Map for Options

// Copy-paste from Option's impls for the most part
impl<A, O> ArchiveWith<Option<O>> for Map<A>
where
    A: ArchiveWith<O>,
{
    type Archived = ArchivedOption<<A as ArchiveWith<O>>::Archived>;
    type Resolver = Option<<A as ArchiveWith<O>>::Resolver>;

    unsafe fn resolve_with(
        field: &Option<O>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        match resolver {
            None => {
                let out = out.cast::<ArchivedOptionVariantNone>();
                ptr::addr_of_mut!((*out).0).write(ArchivedOptionTag::None);
            }
            Some(resolver) => {
                let out =
                    out.cast::<ArchivedOptionVariantSome<
                        <A as ArchiveWith<O>>::Archived,
                    >>();
                ptr::addr_of_mut!((*out).0).write(ArchivedOptionTag::Some);

                let value = if let Some(value) = field.as_ref() {
                    value
                } else {
                    unreachable_unchecked();
                };

                let (fp, fo) = out_field!(out.1);
                A::resolve_with(value, pos + fp, resolver, fo);
            }
        }
    }
}

impl<A, O, S, E> SerializeWith<Option<O>, S, E> for Map<A>
where
    S: ?Sized,
    A: ArchiveWith<O> + SerializeWith<O, S, E>,
{
    fn serialize_with(
        field: &Option<O>,
        s: &mut S,
    ) -> Result<Self::Resolver, E> {
        field
            .as_ref()
            .map(|value| A::serialize_with(value, s))
            .transpose()
    }
}

impl<A, O, D, E>
    DeserializeWith<
        ArchivedOption<<A as ArchiveWith<O>>::Archived>,
        Option<O>,
        D,
        E,
    > for Map<A>
where
    D: ?Sized,
    A: ArchiveWith<O> + DeserializeWith<<A as ArchiveWith<O>>::Archived, O, D, E>,
{
    fn deserialize_with(
        field: &ArchivedOption<<A as ArchiveWith<O>>::Archived>,
        d: &mut D,
    ) -> Result<Option<O>, E> {
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

#[repr(C)]
struct ArchivedOptionVariantNone(ArchivedOptionTag);

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

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

impl<F: Serialize<S, E>, S: ?Sized, E> SerializeWith<&F, S, E> for Inline {
    #[inline]
    fn serialize_with(
        field: &&F,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        field.serialize(serializer)
    }
}

// BoxedInline

impl<F: ArchiveUnsized + ?Sized> ArchiveWith<&F> for BoxedInline {
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

impl<F: SerializeUnsized<S, E> + ?Sized, S: ?Sized, E> SerializeWith<&F, S, E>
    for BoxedInline
{
    #[inline]
    fn serialize_with(
        field: &&F,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedBox::serialize_from_ref(*field, serializer)
    }
}

// Boxed

impl<F: ArchiveUnsized + ?Sized> ArchiveWith<F> for Boxed {
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

impl<F: SerializeUnsized<S, E> + ?Sized, S: ?Sized, E> SerializeWith<F, S, E>
    for Boxed
{
    #[inline]
    fn serialize_with(
        field: &F,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedBox::serialize_from_ref(field, serializer)
    }
}

impl<F: Archive, D: ?Sized, E>
    DeserializeWith<ArchivedBox<F::Archived>, F, D, E> for Boxed
where
    F::Archived: Deserialize<F, D, E>,
{
    #[inline]
    fn deserialize_with(
        field: &ArchivedBox<F::Archived>,
        deserializer: &mut D,
    ) -> Result<F, E> {
        field.get().deserialize(deserializer)
    }
}

// Niche

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

impl<S: ?Sized, E> SerializeWith<Option<NonZeroIsize>, S, E> for Niche {
    #[inline]
    fn serialize_with(
        _: &Option<NonZeroIsize>,
        _: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(())
    }
}

impl<D: ?Sized, E>
    DeserializeWith<ArchivedOptionNonZeroIsize, Option<NonZeroIsize>, D, E>
    for Niche
{
    #[inline]
    fn deserialize_with(
        field: &ArchivedOptionNonZeroIsize,
        _: &mut D,
    ) -> Result<Option<NonZeroIsize>, E> {
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

impl<S: ?Sized, E> SerializeWith<Option<NonZeroUsize>, S, E> for Niche {
    #[inline]
    fn serialize_with(
        _: &Option<NonZeroUsize>,
        _: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(())
    }
}

impl<D: ?Sized, E>
    DeserializeWith<ArchivedOptionNonZeroUsize, Option<NonZeroUsize>, D, E>
    for Niche
{
    #[inline]
    fn deserialize_with(
        field: &ArchivedOptionNonZeroUsize,
        _: &mut D,
    ) -> Result<Option<NonZeroUsize>, E> {
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

impl<F: Serialize<S, E>, S: ?Sized, E> SerializeWith<UnsafeCell<F>, S, E>
    for Unsafe
{
    #[inline]
    fn serialize_with(
        field: &UnsafeCell<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        unsafe { (*field.get()).serialize(serializer) }
    }
}

impl<F: Archive, D: ?Sized, E>
    DeserializeWith<UnsafeCell<F::Archived>, UnsafeCell<F>, D, E> for Unsafe
where
    F::Archived: Deserialize<F, D, E>,
{
    #[inline]
    fn deserialize_with(
        field: &UnsafeCell<F::Archived>,
        deserializer: &mut D,
    ) -> Result<UnsafeCell<F>, E> {
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

impl<F: Serialize<S, E>, S: ?Sized, E> SerializeWith<Cell<F>, S, E>
    for Unsafe
{
    #[inline]
    fn serialize_with(
        field: &Cell<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        unsafe { (*field.as_ptr()).serialize(serializer) }
    }
}

impl<F: Archive, D: ?Sized, E>
    DeserializeWith<Cell<F::Archived>, Cell<F>, D, E> for Unsafe
where
    F::Archived: Deserialize<F, D, E>,
{
    #[inline]
    fn deserialize_with(
        field: &Cell<F::Archived>,
        deserializer: &mut D,
    ) -> Result<Cell<F>, E> {
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

    unsafe fn resolve_with(
        _: &F,
        _: usize,
        _: Self::Resolver,
        _: *mut Self::Archived,
    ) {
    }
}

impl<F, S: ?Sized, E> SerializeWith<F, S, E> for Skip {
    fn serialize_with(_: &F, _: &mut S) -> Result<(), E> {
        Ok(())
    }
}

impl<F: Default, D: ?Sized, E> DeserializeWith<(), F, D, E> for Skip {
    fn deserialize_with(_: &(), _: &mut D) -> Result<F, E> {
        Ok(Default::default())
    }
}
