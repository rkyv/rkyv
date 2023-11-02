use crate::{
    ser::{ScratchSpace, Serializer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Fallible, Serialize,
};

use thin_vec::ThinVec;

impl<T> Archive for ThinVec<T>
where
    T: Archive,
{
    type Archived = ArchivedVec<Archived<T>>;
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

impl<T, S: ScratchSpace + Serializer + ?Sized> Serialize<S> for ThinVec<T>
where
    T: Serialize<S>,
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
