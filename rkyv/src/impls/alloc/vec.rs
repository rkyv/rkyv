use core::cmp;

#[cfg(not(feature = "std"))]
use ::alloc::{alloc, boxed::Box, vec::Vec};
#[cfg(feature = "std")]
use ::std::alloc;
use rancor::{Fallible, ResultExt as _, Source};

use crate::{
    ser::{Allocator, Writer},
    vec::{ArchivedVec, VecResolver},
    Archive, Deserialize, DeserializeUnsized, LayoutRaw, Place, Serialize,
};

impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for ArchivedVec<T> {
    #[inline]
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedVec<U>> for Vec<T> {
    #[inline]
    fn eq(&self, other: &ArchivedVec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: PartialOrd<U>, U> PartialOrd<Vec<U>> for ArchivedVec<T> {
    #[inline]
    fn partial_cmp(&self, other: &Vec<U>) -> Option<cmp::Ordering> {
        let min_len = self.len().min(other.len());
        for i in 0..min_len {
            match self[i].partial_cmp(&other[i]) {
                Some(cmp::Ordering::Equal) => continue,
                result => return result,
            }
        }
        self.len().partial_cmp(&other.len())
    }
}

impl<T: PartialOrd> PartialOrd<ArchivedVec<T>> for Vec<T> {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedVec<T>) -> Option<cmp::Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}

impl<T: Archive> Archive for Vec<T> {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedVec::resolve_from_slice(self.as_slice(), resolver, out);
    }
}

impl<T: Serialize<S>, S: Fallible + Allocator + Writer + ?Sized> Serialize<S>
    for Vec<T>
{
    #[inline]
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
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Vec<T>, D::Error> {
        let metadata = self.as_slice().deserialize_metadata(deserializer)?;
        let layout = <[T] as LayoutRaw>::layout_raw(metadata).into_error()?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc::alloc(layout) }
        } else {
            layout.align() as *mut u8
        };
        let out = ptr_meta::from_raw_parts_mut(data_address.cast(), metadata);
        unsafe {
            self.as_slice().deserialize_unsized(deserializer, out)?;
        }
        unsafe { Ok(Box::<[T]>::from_raw(out).into()) }
    }
}
