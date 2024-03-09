use bytes::{Bytes, BytesMut};
use rancor::Fallible;

use crate::{
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Serialize,
};

impl Archive for Bytes {
    type Archived = ArchivedVec<u8>;
    type Resolver = VecResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_slice(self, pos, resolver, out);
    }
}

impl<S: Fallible + Allocator + Writer + ?Sized> Serialize<S> for Bytes {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self, serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<Bytes, D> for ArchivedVec<Archived<u8>> {
    #[inline]
    fn deserialize(&self, _deserializer: &mut D) -> Result<Bytes, D::Error> {
        let mut result = BytesMut::new();
        result.extend_from_slice(self.as_slice());
        Ok(result.freeze())
    }
}

impl<T: Archive> PartialEq<Bytes> for ArchivedVec<T>
where
    bytes::Bytes: PartialEq<[T]>,
{
    fn eq(&self, other: &Bytes) -> bool {
        other == self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "std"))]
    use alloc::vec;

    use bytes::Bytes;
    use rancor::{Failure, Infallible};

    use crate::{
        access_unchecked, deserialize, ser::Positional as _, vec::ArchivedVec,
    };

    #[test]
    fn bytes() {
        use crate::ser::CoreSerializer;

        let value = Bytes::from(vec![10, 20, 40, 80]);

        let serializer = crate::util::serialize_into::<_, _, Failure>(
            &value,
            CoreSerializer::<256, 256>::default(),
        )
        .unwrap();
        let end = serializer.pos();
        let result = serializer.into_writer().into_inner();
        let archived =
            unsafe { access_unchecked::<ArchivedVec<u8>>(&result[0..end]) };
        assert_eq!(archived, &value);

        let deserialized =
            deserialize::<Bytes, _, Infallible>(archived, &mut ()).unwrap();
        assert_eq!(value, deserialized);
    }
}
