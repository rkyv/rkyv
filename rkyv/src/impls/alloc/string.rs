use crate::{
    string::{ArchivedString, StringResolver},
    Archive,
    Deserialize,
    DeserializeUnsized,
    Fallible,
    Serialize,
    SerializeUnsized,
};
use core::mem::MaybeUninit;

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: StringResolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedString::resolve_from_str(self.as_str(), pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for String
where
    str: SerializeUnsized<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(self.as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<String, D> for ArchivedString
where
    str: DeserializeUnsized<str, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<String, D::Error> {
        unsafe {
            let data_address = self
                .as_str()
                .deserialize_unsized(deserializer, |layout| alloc::alloc::alloc(layout))?;
            let metadata = self.as_str().deserialize_metadata(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::<str>::from_raw(ptr).into())
        }
    }
}