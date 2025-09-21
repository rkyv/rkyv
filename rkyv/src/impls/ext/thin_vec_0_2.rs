use rancor::Fallible;
use thin_vec_0_2::ThinVec;

use crate::{
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Place, Serialize,
};

impl<T> Archive for ThinVec<T>
where
    T: Archive,
{
    type Archived = ArchivedVec<Archived<T>>;
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<T, S> Serialize<S> for ThinVec<T>
where
    T: Serialize<S>,
    S: Allocator + Writer + Fallible + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.as_slice(), serializer)
    }
}

impl<T, D: Fallible + ?Sized> Deserialize<ThinVec<T>, D>
    for ArchivedVec<Archived<T>>
where
    T: Archive,
    Archived<T>: Deserialize<T, D>,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<ThinVec<T>, D::Error> {
        let mut result = ThinVec::with_capacity(self.len());
        for item in self.as_slice() {
            result.push(item.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<T, U> PartialEq<ThinVec<U>> for ArchivedVec<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &ThinVec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T, U> PartialOrd<ThinVec<U>> for ArchivedVec<T>
where
    T: PartialOrd<U>,
{
    fn partial_cmp(&self, other: &ThinVec<U>) -> Option<::core::cmp::Ordering> {
        crate::impls::lexicographical_partial_ord(
            self.as_slice(),
            other.as_slice(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::ThinVec;
    use crate::api::test::roundtrip_with;

    #[test]
    fn roundtrip_thin_vec() {
        roundtrip_with(&ThinVec::<i32>::from_iter([10, 20, 40, 80]), |a, b| {
            assert_eq!(**a, **b)
        });
    }

    #[test]
    fn test_partial_eq() {
        use crate::Archive;

        #[allow(unused)]
        #[derive(Archive)]
        #[rkyv(crate, compare(PartialEq, PartialOrd))]
        struct Inner {
            a: i32,
        }

        #[allow(unused)]
        #[derive(Archive)]
        #[rkyv(crate, compare(PartialEq, PartialOrd))]
        struct Outer {
            a: ThinVec<Inner>,
        }
    }
}
