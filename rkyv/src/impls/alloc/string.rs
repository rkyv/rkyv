use crate::{
    string::{ArchivedString, StringResolver},
    Archive, Deserialize, DeserializeUnsized, Fallible, Serialize, SerializeUnsized,
};
#[cfg(not(feature = "std"))]
use alloc::{alloc, boxed::Box, string::String};
#[cfg(feature = "std")]
use std::alloc;

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
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
                .deserialize_unsized(deserializer, |layout| alloc::alloc(layout))?;
            let metadata = self.as_str().deserialize_metadata(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::<str>::from_raw(ptr).into())
        }
    }
}

impl PartialEq<String> for ArchivedString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl PartialEq<ArchivedString> for String {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), self.as_str())
    }
}
