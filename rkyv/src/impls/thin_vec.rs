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

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<T, S> Serialize<S> for ThinVec<T>
where
    T: Serialize<S>,
    S: Allocator + Writer + Fallible + ?Sized,
{
    #[inline]
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
    #[inline]
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
    use rancor::{Error, Infallible};
    use thin_vec::ThinVec;

    use crate::{access_unchecked, deserialize, to_bytes};

    #[test]
    fn thin_vec() {
        #[cfg(not(feature = "std"))]
        use alloc::vec;

        use crate::Archived;

        let value = ThinVec::from_iter([10, 20, 40, 80]);

        let result = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<Archived<ThinVec<i32>>>(result.as_ref())
        };
        assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);

        let deserialized =
            deserialize::<ThinVec<i32>, _, Infallible>(archived, &mut ())
                .unwrap();
        assert_eq!(value, deserialized);
    }
}
