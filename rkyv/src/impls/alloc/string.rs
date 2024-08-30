use core::cmp::Ordering;

use rancor::{Fallible, Source};

use crate::{
    alloc::string::{String, ToString},
    string::{ArchivedString, StringResolver},
    Archive, Deserialize, DeserializeUnsized, Place, Serialize,
    SerializeUnsized,
};

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedString::resolve_from_str(self.as_str(), resolver, out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for String
where
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(self.as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<String, D> for ArchivedString
where
    str: DeserializeUnsized<str, D>,
{
    fn deserialize(&self, _: &mut D) -> Result<String, D::Error> {
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

impl PartialOrd<String> for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &String) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl PartialOrd<ArchivedString> for String {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedString) -> Option<Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

#[cfg(test)]
mod tests {
    use crate::{alloc::string::ToString, api::test::roundtrip};

    #[test]
    fn roundtrip_string() {
        roundtrip(&"".to_string());
        roundtrip(&"hello world".to_string());
    }

    #[test]
    fn roundtrip_option_string() {
        roundtrip(&Some("".to_string()));
        roundtrip(&Some("hello world".to_string()));
    }

    #[test]
    fn roundtrip_result_string() {
        roundtrip(&Ok::<_, ()>("".to_string()));
        roundtrip(&Ok::<_, ()>("hello world".to_string()));

        roundtrip(&Err::<(), _>("".to_string()));
        roundtrip(&Err::<(), _>("hello world".to_string()));
    }
}
