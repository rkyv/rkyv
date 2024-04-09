use rancor::Fallible;
use smallvec::{Array, SmallVec};

use crate::{
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Serialize,
};

impl<A: Array> Archive for SmallVec<A>
where
    A::Item: Archive,
{
    type Archived = ArchivedVec<Archived<A::Item>>;
    type Resolver = VecResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_slice(self.as_slice(), pos, resolver, out);
    }
}

impl<A, S> Serialize<S> for SmallVec<A>
where
    A: Array,
    A::Item: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    #[inline]
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
    #[inline]
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

#[cfg(test)]
mod tests {
    use rancor::{Failure, Infallible};
    use smallvec::{smallvec, SmallVec};

    use crate::{
        access_unchecked, deserialize, to_bytes, vec::ArchivedVec, Archived,
    };

    #[test]
    fn small_vec() {
        let value: SmallVec<[i32; 10]> = smallvec![10, 20, 40, 80];

        let bytes = to_bytes::<Failure>(&value).unwrap();
        let archived =
            unsafe { access_unchecked::<ArchivedVec<Archived<i32>>>(&bytes) };
        assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);

        let deserialized = deserialize::<SmallVec<[i32; 10]>, _, Infallible>(
            archived,
            &mut (),
        )
        .unwrap();
        assert_eq!(value, deserialized);
    }
}
