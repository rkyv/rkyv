use rancor::Fallible;
use thin_vec::ThinVec;

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

#[cfg(test)]
mod tests {
    use thin_vec::ThinVec;

    use crate::api::test::roundtrip_with;

    #[test]
    fn roundtrip_thin_vec() {
        roundtrip_with(
            &ThinVec::<i32>::from_iter([10, 20, 40, 80].into_iter()),
            |a, b| assert_eq!(**a, **b),
        );
    }
}
