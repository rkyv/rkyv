use rancor::{Fallible, ResultExt as _, Source};

use crate::{
    alloc::{alloc::alloc, boxed::Box, vec::Vec},
    ser::{Allocator, Writer},
    traits::LayoutRaw,
    vec::{ArchivedVec, VecResolver},
    Archive, Deserialize, DeserializeUnsized, Place, Serialize,
};

impl<T: Archive> Archive for Vec<T> {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<T: Serialize<S>, S: Fallible + Allocator + Writer + ?Sized> Serialize<S>
    for Vec<T>
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::<T::Archived>::serialize_from_slice(
            self.as_slice(),
            serializer,
        )
    }
}

impl<T, D> Deserialize<Vec<T>, D> for ArchivedVec<T::Archived>
where
    T: Archive,
    [T::Archived]: DeserializeUnsized<[T], D>,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Vec<T>, D::Error> {
        let metadata = self.as_slice().deserialize_metadata();
        let layout = <[T] as LayoutRaw>::layout_raw(metadata).into_error()?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc(layout) }
        } else {
            crate::polyfill::dangling(&layout).as_ptr()
        };
        let out = ptr_meta::from_raw_parts_mut(data_address.cast(), metadata);
        unsafe {
            self.as_slice().deserialize_unsized(deserializer, out)?;
        }
        unsafe { Ok(Box::<[T]>::from_raw(out).into()) }
    }
}

impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for ArchivedVec<T> {
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: PartialOrd<U>, U> PartialOrd<Vec<U>> for ArchivedVec<T> {
    fn partial_cmp(&self, other: &Vec<U>) -> Option<::core::cmp::Ordering> {
        crate::impls::lexicographical_partial_ord(
            self.as_slice(),
            other.as_slice(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        alloc::{vec, vec::Vec},
        api::test::roundtrip,
    };

    #[test]
    fn roundtrip_vec() {
        roundtrip(&Vec::<i32>::new());
        roundtrip(&vec![1, 2, 3, 4]);
    }

    #[test]
    fn roundtrip_vec_zst() {
        roundtrip(&Vec::<()>::new());
        roundtrip(&vec![(), (), (), ()]);
    }

    #[test]
    fn roundtrip_option_vec() {
        roundtrip(&Some(Vec::<i32>::new()));
        roundtrip(&Some(vec![1, 2, 3, 4]));
    }

    #[test]
    fn roundtrip_result_vec() {
        roundtrip(&Ok::<_, ()>(Vec::<i32>::new()));
        roundtrip(&Ok::<_, ()>(vec![1, 2, 3, 4]));

        roundtrip(&Err::<(), _>(Vec::<i32>::new()));
        roundtrip(&Err::<(), _>(vec![1, 2, 3, 4]));
    }
}
