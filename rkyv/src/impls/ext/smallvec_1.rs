use rancor::Fallible;
use smallvec_1::{Array, SmallVec};

use crate::{
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Place, Serialize,
};

impl<A: Array> Archive for SmallVec<A>
where
    A::Item: Archive,
{
    type Archived = ArchivedVec<Archived<A::Item>>;
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<A, S> Serialize<S> for SmallVec<A>
where
    A: Array,
    A::Item: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.as_slice(), serializer)
    }
}

impl<A, D> Deserialize<SmallVec<A>, D> for ArchivedVec<Archived<A::Item>>
where
    A: Array,
    A::Item: Archive,
    Archived<A::Item>: Deserialize<A::Item, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<SmallVec<A>, D::Error> {
        let mut result = SmallVec::new();
        for item in self.as_slice() {
            result.push(item.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<A, U> PartialEq<SmallVec<A>> for ArchivedVec<U>
where
    A: Array,
    U: PartialEq<A::Item>,
{
    fn eq(&self, other: &SmallVec<A>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T, A> PartialOrd<SmallVec<A>> for ArchivedVec<T>
where
    A: Array,
    T: PartialOrd<A::Item>,
{
    fn partial_cmp(
        &self,
        other: &SmallVec<A>,
    ) -> Option<::core::cmp::Ordering> {
        crate::impls::lexicographical_partial_ord(
            self.as_slice(),
            other.as_slice(),
        )
    }
}

#[cfg(test)]
mod tests {
    use smallvec_1::{smallvec, SmallVec};

    use crate::api::test::roundtrip_with;

    #[test]
    fn roundtrip_small_vec() {
        let value: SmallVec<[i32; 4]> = smallvec![10, 20, 40, 80];
        roundtrip_with(&value, |a, b| assert_eq!(**a, **b));
    }
}
