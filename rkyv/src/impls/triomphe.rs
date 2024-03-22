#[cfg(not(feature = "std"))]
use alloc::{alloc, boxed::Box};
use core::mem::forget;
#[cfg(feature = "std")]
use std::alloc;

use rancor::Fallible;
use triomphe::Arc;

use crate::{
    de::{Metadata, Pooling, PoolingExt, SharedPointer},
    rc::{ArchivedRc, RcResolver},
    ser::{Sharing, Writer},
    Archive, ArchiveUnsized, Deserialize, DeserializeUnsized, Serialize,
    SerializeUnsized,
};

pub struct TriompheArcFlavor;

unsafe impl<T> SharedPointer<T> for Arc<T> {
    #[inline]
    unsafe fn from_value(ptr: *mut T) -> *mut T {
        let arc = Arc::<T>::from(unsafe { Box::from_raw(ptr) });
        Arc::into_raw(arc).cast_mut()
    }

    #[inline]
    unsafe fn drop(ptr: *mut T) {
        drop(unsafe { Arc::from_raw(ptr) })
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for Arc<T> {
    type Archived = ArchivedRc<T::Archived, TriompheArcFlavor>;
    type Resolver = RcResolver;

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
    S: Writer + Sharing + Fallible + ?Sized,
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

impl<T, D> Deserialize<Arc<T>, D> for ArchivedRc<T::Archived, TriompheArcFlavor>
where
    T: ArchiveUnsized + 'static,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    T::Archived: DeserializeUnsized<T, D>,
    D: Pooling + Fallible + ?Sized,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Arc<T>, D::Error> {
        let raw_shared_ptr = deserializer.deserialize_shared::<_, Arc<T>, _>(
            self.get(),
            // TODO: make sure that Arc<()> won't alloc with zero size layouts
            |layout| unsafe { alloc::alloc(layout) },
        )?;
        let shared_ptr = unsafe { Arc::<T>::from_raw(raw_shared_ptr) };
        forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}
