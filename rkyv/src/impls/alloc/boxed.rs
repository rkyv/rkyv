use crate::{
    boxed::{ArchivedBox, BoxResolver},
    Archive,
    ArchiveUnsized,
    Deserialize,
    DeserializeUnsized,
    Fallible,
    Serialize,
    SerializeUnsized,
};
use core::mem::MaybeUninit;

impl<T: ArchiveUnsized + ?Sized> Archive for Box<T> {
    type Archived = ArchivedBox<T::Archived>;
    type Resolver = BoxResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedBox::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T: SerializeUnsized<S> + ?Sized, S: Fallible + ?Sized> Serialize<S> for Box<T> {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedBox::serialize_from_ref(self.as_ref(), serializer)
    }
}

impl<T: ArchiveUnsized + ?Sized, D: Fallible + ?Sized> Deserialize<Box<T>, D> for ArchivedBox<T::Archived>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Box<T>, D::Error> {
        unsafe {
            let data_address = self
                .get()
                .deserialize_unsized(deserializer, |layout| alloc::alloc::alloc(layout))?;
            let metadata = self.get().deserialize_metadata(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::from_raw(ptr))
        }
    }
}