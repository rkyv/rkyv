use crate::{
    ser::{ScratchSpace, Serializer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Fallible, Serialize,
};
use smallvec::{Array, SmallVec};

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
    S: Fallible + ScratchSpace + Serializer + ?Sized,
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
    use crate::{access_unchecked, ser::Serializer, Deserialize};
    use rancor::{Strategy, Infallible, Failure};
    use smallvec::{smallvec, SmallVec};

    #[test]
    fn small_vec() {
        use crate::ser::serializers::CoreSerializer;

        let value: SmallVec<[i32; 10]> = smallvec![10, 20, 40, 80];

        let serializer = crate::util::serialize_into::<_, _, Failure>(
            &value,
            CoreSerializer::<256, 256>::default(),
        ).unwrap();
        let end = Serializer::<Failure>::pos(&serializer);
        let result = serializer.into_serializer().into_inner();
        let archived =
            unsafe { access_unchecked::<SmallVec<[i32; 10]>>(&result[0..end]) };
        assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);

        let deserialized: SmallVec<[i32; 10]> =
            archived.deserialize(Strategy::<_, Infallible>::wrap(&mut ())).unwrap();
        assert_eq!(value, deserialized);
    }
}
