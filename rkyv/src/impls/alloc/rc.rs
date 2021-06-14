use crate::{
    de::{SharedDeserializer, SharedPointer},
    rc::{ArchivedRc, RcResolver, ArchivedRcWeak, RcWeakResolver},
    ser::SharedSerializer,
    Archive,
    ArchivePointee,
    ArchiveUnsized,
    Deserialize,
    DeserializeUnsized,
    Serialize,
    SerializeUnsized,
};
use core::mem::{forget, MaybeUninit};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{boxed::Box, rc, sync};
#[cfg(feature = "std")]
use std::{rc, sync};

// Rc

impl<T: ?Sized> SharedPointer for rc::Rc<T> {
    #[inline]
    fn data_address(&self) -> *const () {
        rc::Rc::as_ptr(self) as *const ()
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Rc<T> {
    type Archived = ArchivedRc<T::Archived>;
    type Resolver = RcResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedRc::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S>
    for rc::Rc<T>
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived>::serialize_from_ref(self.as_ref(), serializer)
    }
}

impl<T: ArchiveUnsized + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<rc::Rc<T>, D>
    for ArchivedRc<T::Archived>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<rc::Rc<T>, D::Error> {
        let raw_shared_ptr = deserializer
            .deserialize_shared::<T, rc::Rc<T>, _>(self.get(), |ptr| {
                rc::Rc::<T>::from(unsafe { Box::from_raw(ptr) })
            })?;
        let shared_ptr = unsafe { rc::Rc::<T>::from_raw(raw_shared_ptr) };
        forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<rc::Rc<U>> for ArchivedRc<T> {
    #[inline]
    fn eq(&self, other: &rc::Rc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// rc::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived>;
    type Resolver = RcWeakResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedRcWeak::resolve_from_ref(self.upgrade().as_ref().map(|v| v.as_ref()), pos, resolver, out);
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S>
    for rc::Weak<T>
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::serialize_from_ref(self.upgrade().as_ref().map(|v| v.as_ref()), serializer)
    }
}

// Deserialize can only be implemented for sized types because weak pointers don't have from/into
// raw functions.
impl<T: Archive + 'static, D: SharedDeserializer + ?Sized> Deserialize<rc::Weak<T>, D>
    for ArchivedRcWeak<T::Archived>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<rc::Weak<T>, D::Error> {
        Ok(match self {
            ArchivedRcWeak::None => rc::Weak::new(),
            ArchivedRcWeak::Some(r) => rc::Rc::downgrade(&r.deserialize(deserializer)?),
        })
    }
}

// Arc

impl<T: ?Sized> SharedPointer for sync::Arc<T> {
    #[inline]
    fn data_address(&self) -> *const () {
        sync::Arc::as_ptr(self) as *const ()
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Arc<T> {
    type Archived = ArchivedRc<T::Archived>;
    type Resolver = RcResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedRc::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S>
    for sync::Arc<T>
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived>::serialize_from_ref(self.as_ref(), serializer)
    }
}

impl<T: ArchiveUnsized + ?Sized + 'static, D: SharedDeserializer + ?Sized>
    Deserialize<sync::Arc<T>, D> for ArchivedRc<T::Archived>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<sync::Arc<T>, D::Error> {
        let raw_shared_ptr = deserializer.deserialize_shared(self.get(), |ptr| {
            sync::Arc::<T>::from(unsafe { Box::from_raw(ptr) })
        })?;
        let shared_ptr = unsafe { sync::Arc::<T>::from_raw(raw_shared_ptr) };
        forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<sync::Arc<U>>
    for ArchivedRc<T>
{
    #[inline]
    fn eq(&self, other: &sync::Arc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// sync::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived>;
    type Resolver = RcWeakResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedRcWeak::resolve_from_ref(self.upgrade().as_ref().map(|v| v.as_ref()), pos, resolver, out);
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S>
    for sync::Weak<T>
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::serialize_from_ref(self.upgrade().as_ref().map(|v| v.as_ref()), serializer)
    }
}

// Deserialize can only be implemented for sized types because weak pointers don't have from/into
// raw functions.
impl<T: Archive + 'static, D: SharedDeserializer + ?Sized> Deserialize<sync::Weak<T>, D>
    for ArchivedRcWeak<T::Archived>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<sync::Weak<T>, D::Error> {
        Ok(match self {
            ArchivedRcWeak::None => sync::Weak::new(),
            ArchivedRcWeak::Some(r) => sync::Arc::downgrade(&r.deserialize(deserializer)?),
        })
    }
}
