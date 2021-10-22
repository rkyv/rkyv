use crate::{
    boxed::{ArchivedBox, BoxResolver},
    with::{ArchiveWith, AsBox, DeserializeWith, Inline, RefAsBox, SerializeWith},
    Archive, ArchiveUnsized, Deserialize, Fallible, Serialize, SerializeUnsized,
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
