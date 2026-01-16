use arrayvec_0_7::ArrayString;
use rancor::{Fallible, Source};

use crate::{
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    Archive, Deserialize, Place, Serialize,
};

impl<const CAP: usize> Archive for ArrayString<CAP> {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(self, resolver, out);
    }
}

impl<S, const CAP: usize> Serialize<S> for ArrayString<CAP>
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

impl<D: Fallible + ?Sized, const CAP: usize> Deserialize<ArrayString<CAP>, D>
    for ArchivedString
{
    fn deserialize(
        &self,
        _deserializer: &mut D,
    ) -> Result<ArrayString<CAP>, D::Error> {
        Ok(ArrayString::from(self.as_str()).unwrap())
    }
}

impl<const CAP: usize> PartialEq<ArrayString<CAP>> for ArchivedString {
    fn eq(&self, other: &ArrayString<CAP>) -> bool {
        other.as_str() == self.as_str()
    }
}

impl<const CAP: usize> PartialOrd<ArrayString<CAP>> for ArchivedString {
    fn partial_cmp(
        &self,
        other: &ArrayString<CAP>,
    ) -> Option<::core::cmp::Ordering> {
        Some(self.as_str().cmp(other.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::ArrayString;
    use crate::api::test::roundtrip;

    #[test]
    fn roundtrip_arraystring() {
        roundtrip(&ArrayString::<100>::from("arraystring test!").unwrap());
        roundtrip(&ArrayString::<100>::from("").unwrap());
    }
}
