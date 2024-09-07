use rancor::Fallible;
#[cfg(feature = "alloc")]
use tinyvec_1::TinyVec;
use tinyvec_1::{Array, ArrayVec, SliceVec};

use crate::{
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Archived, Deserialize, Place, Serialize,
};

// ArrayVec

impl<A: Array> Archive for ArrayVec<A>
where
    A::Item: Archive,
{
    type Archived = ArchivedVec<Archived<A::Item>>;
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<A, S> Serialize<S> for ArrayVec<A>
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

impl<A, D> Deserialize<ArrayVec<A>, D> for ArchivedVec<Archived<A::Item>>
where
    A: Array,
    A::Item: Archive,
    Archived<A::Item>: Deserialize<A::Item, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<ArrayVec<A>, D::Error> {
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
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<'s, T, S> Serialize<S> for SliceVec<'s, T>
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
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
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

#[cfg(feature = "alloc")]
impl<A, S> Serialize<S> for TinyVec<A>
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

#[cfg(feature = "alloc")]
impl<A, D> Deserialize<TinyVec<A>, D> for ArchivedVec<Archived<A::Item>>
where
    A: Array,
    A::Item: Archive,
    Archived<A::Item>: Deserialize<A::Item, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<TinyVec<A>, D::Error> {
        let mut result = TinyVec::new();
        for item in self.as_slice() {
            result.push(item.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<T, A> PartialEq<ArrayVec<A>> for ArchivedVec<T>
where
    A: Array,
    T: PartialEq<A::Item>,
{
    fn eq(&self, other: &ArrayVec<A>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T, A> PartialOrd<ArrayVec<A>> for ArchivedVec<T>
where
    A: Array,
    T: PartialOrd<A::Item>,
{
    fn partial_cmp(
        &self,
        other: &ArrayVec<A>,
    ) -> Option<::core::cmp::Ordering> {
        crate::impls::lexicographical_partial_ord(
            self.as_slice(),
            other.as_slice(),
        )
    }
}

impl<T, U> PartialEq<SliceVec<'_, U>> for ArchivedVec<T>
where
    T: PartialEq<U>,
{
    fn eq(&self, other: &SliceVec<'_, U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T, U> PartialOrd<SliceVec<'_, U>> for ArchivedVec<T>
where
    T: PartialOrd<U>,
{
    fn partial_cmp(
        &self,
        other: &SliceVec<'_, U>,
    ) -> Option<::core::cmp::Ordering> {
        crate::impls::lexicographical_partial_ord(
            self.as_slice(),
            other.as_slice(),
        )
    }
}

#[cfg(test)]
mod tests {
    use tinyvec_1::{array_vec, Array, SliceVec};

    use crate::api::test::{roundtrip_with, to_archived};

    #[test]
    fn roundtrip_array_vec() {
        roundtrip_with(&array_vec!([i32; 10] => 10, 20, 40, 80), |a, b| {
            assert_eq!(**a, **b)
        });
    }

    #[test]
    fn serialize_slice_vec() {
        let mut backing = [0i32; 10];
        let mut value = SliceVec::from_slice_len(backing.as_slice_mut(), 0);
        value.push(10);
        value.push(20);
        value.push(40);
        value.push(80);

        to_archived(&value, |archived| {
            assert_eq!(archived.as_slice(), &[10, 20, 40, 80]);
        });
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn roundtrip_tiny_vec() {
        use tinyvec_1::tiny_vec;

        use crate::alloc::vec;

        roundtrip_with(&tiny_vec!([i32; 10] => 10, 20, 40, 80), |a, b| {
            assert_eq!(**a, **b)
        });
    }
}
