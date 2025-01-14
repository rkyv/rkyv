use core::alloc::LayoutError;

use ptr_meta::{from_raw_parts_mut, Pointee};
use rancor::{Fallible, Source};

use crate::{
    alloc::{alloc::alloc, boxed::Box, sync},
    de::{FromMetadata, Metadata, Pooling, PoolingExt as _, SharedPointer},
    rc::{ArcFlavor, ArchivedRc, ArchivedRcWeak, RcResolver, RcWeakResolver},
    ser::{Sharing, Writer},
    traits::{ArchivePointee, LayoutRaw},
    Archive, ArchiveUnsized, Deserialize, DeserializeUnsized, Place, Serialize,
    SerializeUnsized,
};

// Arc

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Arc<T> {
    type Archived = ArchivedRc<T::Archived, ArcFlavor>;
    type Resolver = RcResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedRc::resolve_from_ref(self.as_ref(), resolver, out);
    }
}

impl<T, S> Serialize<S> for sync::Arc<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

unsafe impl<T: LayoutRaw + Pointee + ?Sized> SharedPointer<T> for sync::Arc<T> {
    fn alloc(metadata: T::Metadata) -> Result<*mut T, LayoutError> {
        let layout = T::layout_raw(metadata)?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc(layout) }
        } else {
            crate::polyfill::dangling(&layout).as_ptr()
        };
        let ptr = from_raw_parts_mut(data_address.cast(), metadata);
        Ok(ptr)
    }

    unsafe fn from_value(ptr: *mut T) -> *mut T {
        let arc = sync::Arc::<T>::from(unsafe { Box::from_raw(ptr) });
        sync::Arc::into_raw(arc).cast_mut()
    }

    unsafe fn drop(ptr: *mut T) {
        drop(unsafe { sync::Arc::from_raw(ptr) });
    }
}

impl<T, D> Deserialize<sync::Arc<T>, D> for ArchivedRc<T::Archived, ArcFlavor>
where
    T: ArchiveUnsized + LayoutRaw + Pointee + ?Sized + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata> + FromMetadata,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Arc<T>, D::Error> {
        let raw_shared_ptr =
            deserializer.deserialize_shared::<_, sync::Arc<T>>(self.get())?;
        unsafe {
            sync::Arc::<T>::increment_strong_count(raw_shared_ptr);
        }
        unsafe { Ok(sync::Arc::<T>::from_raw(raw_shared_ptr)) }
    }
}

impl<T, U> PartialEq<sync::Arc<U>> for ArchivedRc<T, ArcFlavor>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    fn eq(&self, other: &sync::Arc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// sync::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, ArcFlavor>;
    type Resolver = RcWeakResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            resolver,
            out,
        );
    }
}

impl<T, S> Serialize<S> for sync::Weak<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

// Deserialize can only be implemented for sized types because weak pointers
// don't have from/into raw functions.
impl<T, D> Deserialize<sync::Weak<T>, D>
    for ArchivedRcWeak<T::Archived, ArcFlavor>
where
    // Deserialize can only be implemented for sized types because weak pointers
    // to unsized types don't have `new` functions.
    T: ArchiveUnsized
        + LayoutRaw
        + Pointee // + ?Sized
        + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata> + FromMetadata,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Weak<T>, D::Error> {
        Ok(match self.upgrade() {
            None => sync::Weak::new(),
            Some(r) => sync::Arc::downgrade(&r.deserialize(deserializer)?),
        })
    }
}
