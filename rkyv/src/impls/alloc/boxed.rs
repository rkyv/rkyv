use core::cmp;

use rancor::{Fallible, ResultExt as _, Source};

use crate::{
    alloc::{alloc::alloc, boxed::Box},
    boxed::{ArchivedBox, BoxResolver},
    niche::option_box::ArchivedOptionBox,
    traits::{ArchivePointee, LayoutRaw},
    Archive, ArchiveUnsized, Deserialize, DeserializeUnsized, Place, Serialize,
    SerializeUnsized,
};

impl<T: ArchiveUnsized + ?Sized> Archive for Box<T> {
    type Archived = ArchivedBox<T::Archived>;
    type Resolver = BoxResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedBox::resolve_from_ref(self.as_ref(), resolver, out);
    }
}

impl<T, S> Serialize<S> for Box<T>
where
    T: SerializeUnsized<S> + ?Sized,
    S: Fallible + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedBox::serialize_from_ref(self.as_ref(), serializer)
    }
}

impl<T, D> Deserialize<Box<T>, D> for ArchivedBox<T::Archived>
where
    T: ArchiveUnsized + LayoutRaw + ?Sized,
    T::Archived: DeserializeUnsized<T, D>,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Box<T>, D::Error> {
        let metadata = self.get().deserialize_metadata();
        let layout = T::layout_raw(metadata).into_error()?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc(layout) }
        } else {
            crate::polyfill::dangling(&layout).as_ptr()
        };

        let out = ptr_meta::from_raw_parts_mut(data_address.cast(), metadata);

        unsafe {
            self.get().deserialize_unsized(deserializer, out)?;
        }
        unsafe { Ok(Box::from_raw(out)) }
    }
}

impl<T, U> PartialEq<Box<U>> for ArchivedBox<T>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    fn eq(&self, other: &Box<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

impl<T, U> PartialOrd<Box<U>> for ArchivedBox<T>
where
    T: ArchivePointee + PartialOrd<U> + ?Sized,
    U: ?Sized,
{
    fn partial_cmp(&self, other: &Box<U>) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.as_ref())
    }
}

impl<T, U> PartialEq<Option<Box<T>>> for ArchivedOptionBox<U>
where
    T: ?Sized,
    U: ArchivePointee + PartialEq<T> + ?Sized,
{
    fn eq(&self, other: &Option<Box<T>>) -> bool {
        match (self.as_deref(), other.as_deref()) {
            (Some(self_value), Some(other_value)) => self_value.eq(other_value),
            (None, None) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        alloc::{boxed::Box, string::ToString, vec, vec::Vec},
        api::test::roundtrip,
    };

    #[test]
    fn roundtrip_box() {
        roundtrip(&Box::new(42));
        roundtrip(&Box::new([1, 2, 3, 4, 5, 6]));
    }

    #[test]
    fn roundtrip_boxed_str() {
        roundtrip(&"".to_string().into_boxed_str());
        roundtrip(&"hello world".to_string().into_boxed_str());
    }

    #[test]
    fn roundtrip_boxed_slice() {
        roundtrip(&Vec::<i32>::new().into_boxed_slice());
        roundtrip(&vec![1, 2, 3, 4].into_boxed_slice());
    }

    #[test]
    fn roundtrip_box_zsts() {
        roundtrip(&Box::new(()));
        roundtrip(&Vec::<()>::new().into_boxed_slice());
        roundtrip(&vec![(), (), (), ()].into_boxed_slice());
    }

    #[test]
    fn roundtrip_option_box() {
        roundtrip(&Some(Box::new(42)));
        roundtrip(&Some(Box::new([1, 2, 3, 4, 5, 6])));
    }

    #[test]
    fn roundtrip_option_box_str() {
        roundtrip(&Some("".to_string().into_boxed_str()));
        roundtrip(&Some("hello world".to_string().into_boxed_str()));
    }

    #[test]
    fn roundtrip_option_box_slice() {
        roundtrip(&Some(Vec::<i32>::new().into_boxed_slice()));
        roundtrip(&Some(vec![1, 2, 3, 4].into_boxed_slice()));
    }

    #[test]
    fn roundtrip_result_box() {
        roundtrip(&Ok::<_, ()>(Box::new(42)));
        roundtrip(&Ok::<_, ()>(Box::new([1, 2, 3, 4, 5, 6])));

        roundtrip(&Err::<(), _>(Box::new(42)));
        roundtrip(&Err::<(), _>(Box::new([1, 2, 3, 4, 5, 6])));
    }

    #[test]
    fn roundtrip_result_box_str() {
        roundtrip(&Ok::<_, ()>("".to_string().into_boxed_str()));
        roundtrip(&Ok::<_, ()>("hello world".to_string().into_boxed_str()));

        roundtrip(&Err::<(), _>("".to_string().into_boxed_str()));
        roundtrip(&Err::<(), _>("hello world".to_string().into_boxed_str()));
    }

    #[test]
    fn roundtrip_result_box_slice() {
        roundtrip(&Ok::<_, ()>(Vec::<i32>::new().into_boxed_slice()));
        roundtrip(&Ok::<_, ()>(vec![1, 2, 3, 4].into_boxed_slice()));

        roundtrip(&Err::<(), _>(Vec::<i32>::new().into_boxed_slice()));
        roundtrip(&Err::<(), _>(vec![1, 2, 3, 4].into_boxed_slice()));
    }
}
