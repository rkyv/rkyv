use crate::{
    de::{SharedDeserializeRegistry, SharedPointer},
    rc::{ArchivedRc, RcResolver},
    ser::{Serializer, SharedSerializeRegistry},
    Archive, ArchiveUnsized, Deserialize, DeserializeUnsized, Serialize,
    SerializeUnsized,
};

#[cfg(not(feature = "std"))]
use alloc::{alloc, boxed::Box};
#[cfg(feature = "std")]
use std::alloc;

use core::mem::forget;

use triomphe::Arc;

pub struct TriompheArcFlavor;

impl<T: ?Sized> SharedPointer for Arc<T> {
    #[inline]
    fn data_address(&self) -> *const () {
        Arc::as_ptr(self) as *const ()
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for Arc<T> {
    type Archived = ArchivedRc<T::Archived, TriompheArcFlavor>;
    type Resolver = RcResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRc::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T, S> Serialize<S> for Arc<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Serializer + SharedSerializeRegistry + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived, TriompheArcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

impl<T: ArchiveUnsized + 'static, D: SharedDeserializeRegistry + ?Sized>
    Deserialize<Arc<T>, D> for ArchivedRc<T::Archived, TriompheArcFlavor>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Arc<T>, D::Error> {
        let raw_shared_ptr = deserializer.deserialize_shared(
            self.get(),
            |ptr| Arc::<T>::from(unsafe { Box::from_raw(ptr) }),
            |layout| unsafe { alloc::alloc(layout) },
        )?;
        let shared_ptr = unsafe { Arc::<T>::from_raw(raw_shared_ptr) };
        forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}
