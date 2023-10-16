use crate::{
    de::{SharedDeserializeRegistry, SharedPointer},
    rc::{ArchivedRc, ArchivedRcWeak, RcResolver, RcWeakResolver},
    ser::{Serializer, SharedSerializeRegistry},
    Archive, ArchivePointee, ArchiveUnsized, Deserialize, DeserializeUnsized,
    Serialize, SerializeUnsized,
};
#[cfg(not(feature = "std"))]
use alloc::{alloc, boxed::Box, rc, sync};
use core::mem::forget;
#[cfg(feature = "std")]
use std::{alloc, rc, sync};

// Rc

// TODO: move these flavors into a public path

/// The flavor type for `Rc`.
pub struct RcFlavor;

impl<T: ?Sized> SharedPointer for rc::Rc<T> {
    #[inline]
    fn data_address(&self) -> *const () {
        rc::Rc::as_ptr(self) as *const ()
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Rc<T> {
    type Archived = ArchivedRc<T::Archived, RcFlavor>;
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

impl<T, S, E> Serialize<S, E> for rc::Rc<T>
where
    T: SerializeUnsized<S, E> + ?Sized + 'static,
    S: Serializer<E> + SharedSerializeRegistry<E> + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedRc::<T::Archived, RcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

impl<T, D, E> Deserialize<rc::Rc<T>, D, E> for ArchivedRc<T::Archived, RcFlavor>
where
    T: ArchiveUnsized + ?Sized + 'static,
    T::Archived: DeserializeUnsized<T, D, E>,
    D: SharedDeserializeRegistry<E> + ?Sized,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<rc::Rc<T>, E> {
        let raw_shared_ptr = deserializer.deserialize_shared(
            self.get(),
            |ptr| rc::Rc::<T>::from(unsafe { Box::from_raw(ptr) }),
            |layout| unsafe { alloc::alloc(layout) },
        )?;
        let shared_ptr = unsafe { rc::Rc::<T>::from_raw(raw_shared_ptr) };
        forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<rc::Rc<U>>
    for ArchivedRc<T, RcFlavor>
{
    #[inline]
    fn eq(&self, other: &rc::Rc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// rc::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, RcFlavor>;
    type Resolver = RcWeakResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            pos,
            resolver,
            out,
        );
    }
}

impl<T, S, E> Serialize<S, E> for rc::Weak<T>
where
    T: SerializeUnsized<S, E> + ?Sized + 'static,
    S: Serializer<E> + SharedSerializeRegistry<E> + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedRcWeak::<T::Archived, RcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

// Deserialize can only be implemented for sized types because weak pointers don't have from/into
// raw functions.
impl<T, D, E> Deserialize<rc::Weak<T>, D, E> for ArchivedRcWeak<T::Archived, RcFlavor>
where
    T: Archive + 'static,
    T::Archived: DeserializeUnsized<T, D, E>,
    D: SharedDeserializeRegistry<E> + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<rc::Weak<T>, E> {
        Ok(match self {
            ArchivedRcWeak::None => rc::Weak::new(),
            ArchivedRcWeak::Some(r) => {
                rc::Rc::downgrade(&r.deserialize(deserializer)?)
            }
        })
    }
}

// Arc

/// The flavor type for `Arc`.
pub struct ArcFlavor;

impl<T: ?Sized> SharedPointer for sync::Arc<T> {
    #[inline]
    fn data_address(&self) -> *const () {
        sync::Arc::as_ptr(self) as *const ()
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Arc<T> {
    type Archived = ArchivedRc<T::Archived, ArcFlavor>;
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

impl<T, S, E> Serialize<S, E> for sync::Arc<T>
where
    T: SerializeUnsized<S, E> + ?Sized + 'static,
    S: Serializer<E> + SharedSerializeRegistry<E> + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedRc::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

impl<
    T: ArchiveUnsized + ?Sized + 'static,
    D: SharedDeserializeRegistry<E> + ?Sized,
    E,
> Deserialize<sync::Arc<T>, D, E> for ArchivedRc<T::Archived, ArcFlavor>
where
    T::Archived: DeserializeUnsized<T, D, E>,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Arc<T>, E> {
        let raw_shared_ptr = deserializer.deserialize_shared(
            self.get(),
            |ptr| sync::Arc::<T>::from(unsafe { Box::from_raw(ptr) }),
            |layout| unsafe { alloc::alloc(layout) },
        )?;
        let shared_ptr = unsafe { sync::Arc::<T>::from_raw(raw_shared_ptr) };
        forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}

impl<T, U> PartialEq<sync::Arc<U>> for ArchivedRc<T, ArcFlavor>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    #[inline]
    fn eq(&self, other: &sync::Arc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// sync::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, ArcFlavor>;
    type Resolver = RcWeakResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            pos,
            resolver,
            out,
        );
    }
}

impl<T, S, E> Serialize<S, E> for sync::Weak<T>
where
    T: SerializeUnsized<S, E> + ?Sized + 'static,
    S: Serializer<E> + SharedSerializeRegistry<E> + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedRcWeak::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

// Deserialize can only be implemented for sized types because weak pointers don't have from/into
// raw functions.
impl<T, D, E> Deserialize<sync::Weak<T>, D, E>
    for ArchivedRcWeak<T::Archived, ArcFlavor>
where
    T: Archive + 'static,
    T::Archived: DeserializeUnsized<T, D, E>,
    D: SharedDeserializeRegistry<E> + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Weak<T>, E> {
        Ok(match self {
            ArchivedRcWeak::None => sync::Weak::new(),
            ArchivedRcWeak::Some(r) => {
                sync::Arc::downgrade(&r.deserialize(deserializer)?)
            }
        })
    }
}
