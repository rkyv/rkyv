use crate::{
    string::{ArchivedString, StringResolver},
    Archive, Deserialize, DeserializeUnsized, Serialize,
    SerializeUnsized,
};
#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};
use core::cmp::Ordering;

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedString::resolve_from_str(self.as_str(), pos, resolver, out);
    }
}

impl<S: ?Sized, E> Serialize<S, E> for String
where
    str: SerializeUnsized<S, E>,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedString::serialize_from_str(self.as_str(), serializer)
    }
}

impl<D: ?Sized, E> Deserialize<String, D, E> for ArchivedString
where
    str: DeserializeUnsized<str, D, E>,
{
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<String, E> {
        Ok(self.as_str().to_string())
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

impl PartialOrd<ArchivedString> for String {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedString) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl PartialOrd<String> for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}
