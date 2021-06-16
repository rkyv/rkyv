//! [`Archive`](crate::Archive) implementations for `tinyvec` types.

use crate::{
    ser::Serializer,
    vec::{ArchivedVec, VecResolver},
    Archive,
    Archived,
    Deserialize,
    Fallible,
    MetadataResolver,
    Serialize,
};
use core::mem::MaybeUninit;
use tinyvec::{ArrayVec, SliceVec};
#[cfg(feature = "alloc")]
use tinyvec::{Array, TinyVec};

// ArrayVec

impl<A: Array> Archive for ArrayVec<A>
where
    A::Item: Archive,
{
    type Archived = ArchivedVec<Archived<A::Item>>;
    type Resolver = VecResolver<MetadataResolver<[A::Item]>>;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_slice(self.as_slice(), pos, resolver, out);
    }
}

impl<A: Array, S: Serializer + ?Sized> Serialize<S> for ArrayVec<A>
where
    A::Item: Serialize<S>,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.as_slice(), serializer)
    }
}

impl<A: Array, D: Fallible + ?Sized> Deserialize<ArrayVec<A>, D> for ArchivedVec<Archived<A::Item>>
where
    A::Item: Archive,
    Archived<A::Item>: Deserialize<A::Item, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<ArrayVec<A>, D::Error> {
        let mut result = ArrayVec::new();
        for item in self.as_slice() {
            result.push(item.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

// SliceVec

impl<'s, T: Archive> Archive for SliceVec<'s, T> {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver<MetadataResolver<[T]>>;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_slice(self.as_slice(), pos, resolver, out);
    }
}

impl<'s, T: Serialize<S>, S: Serializer + ?Sized> Serialize<S> for SliceVec<'s, T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.as_slice(), serializer)
    }
}

// SliceVec cannot be deserialized because it borrows backing memory

// TinyVec

#[cfg(feature = "alloc")]
impl<A: Array> Archive for TinyVec<A>
where
    A::Item: Archive,
{
    type Archived = ArchivedVec<Archived<A::Item>>;
    type Resolver = VecResolver<MetadataResolver<[A::Item]>>;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_slice(self.as_slice(), pos, resolver, out);
    }
}

#[cfg(feature = "alloc")]
impl<A: Array, S: Serializer + ?Sized> Serialize<S> for TinyVec<A>
where
    A::Item: Serialize<S>,
{
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(self.as_slice(), serializer)
    }
}

#[cfg(feature = "alloc")]
impl<A: Array, D: Fallible + ?Sized> Deserialize<TinyVec<A>, D> for ArchivedVec<Archived<A::Item>>
where
    A::Item: Archive,
    Archived<A::Item>: Deserialize<A::Item, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<TinyVec<A>, D::Error> {
        let mut result = TinyVec::new();
        for item in self.as_slice() {
            result.push(item.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        archived_root, 
        ser::{Serializer, serializers::AlignedSerializer},
        util::AlignedVec,
        Deserialize,
        Infallible,
    };
    use tinyvec::{array_vec, ArrayVec, SliceVec};
    #[cfg(feature = "alloc")]
    use tinyvec::{tiny_vec, Array, TinyVec};

    #[test]
    fn array_vec() {
        let value = array_vec!([i32; 10] => 10, 20, 40, 80);

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<ArrayVec<[i32; 10]>>(result.as_ref()) };
        assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);

        let deserialized = Deserialize::<ArrayVec<[i32; 10]>, _>::deserialize(
            archived,
            &mut Infallible,
        ).unwrap();
        assert_eq!(value, deserialized);
    }

    #[test]
    fn slice_vec() {
        let mut backing = [0i32; 10];
        let mut value = SliceVec::from_slice_len(backing.as_slice_mut(), 0);
        value.push(10);
        value.push(20);
        value.push(40);
        value.push(80);

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<SliceVec<'_, i32>>(result.as_ref()) };
        assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn tiny_vec() {
        let value = tiny_vec!([i32; 10] => 10, 20, 40, 80);

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<TinyVec<[i32; 10]>>(result.as_ref()) };
        assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);

        let deserialized = Deserialize::<TinyVec<[i32; 10]>, _>::deserialize(
            archived,
            &mut Infallible,
        ).unwrap();
        assert_eq!(value, deserialized);
    }
}