use rancor::{Fallible, Source};
use smol_str_0_3::SmolStr;

use crate::{
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    Archive, Deserialize, Place, Serialize,
};

impl Archive for SmolStr {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(self, resolver, out);
    }
}

impl<S> Serialize<S> for SmolStr
where
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(self, serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<SmolStr, D> for ArchivedString {
    fn deserialize(&self, _deserializer: &mut D) -> Result<SmolStr, D::Error> {
        Ok(SmolStr::new(self.as_str()))
    }
}

impl PartialEq<SmolStr> for ArchivedString {
    fn eq(&self, other: &SmolStr) -> bool {
        other.as_str() == self.as_str()
    }
}

impl PartialOrd<SmolStr> for ArchivedString {
    fn partial_cmp(&self, other: &SmolStr) -> Option<::core::cmp::Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::SmolStr;
    use crate::api::test::roundtrip;

    #[test]
    fn roundtrip_smol_str() {
        roundtrip(&SmolStr::new("smol_str"));
    }
}
