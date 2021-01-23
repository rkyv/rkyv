//! [`Archive`] implementation for shared pointers.

#[cfg(feature = "validation")]
pub mod validation;

use core::{ops::{Deref, DerefMut}, pin::Pin};
use std::{rc::Rc, sync::Arc};
use crate::{
    de::SharedDeserializer,
    ser::SharedSerializer,
    Archive,
    Archived,
    ArchiveRef,
    Deserialize,
    DeserializeRef,
    Serialize,
    SerializeRef,
};

/// The resolver for [`Rc`].
pub struct RcResolver(usize);

/// An archived [`Rc`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived. Multiple `ArchivedRc` may point to the same value.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedRc<T>(T);

impl<T: DerefMut> ArchivedRc<T> {
    /// Gets the value of this archived `Rc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` pointers to the same value must not be
    /// dereferenced for the duration of the returned borrow.
    pub unsafe fn get_pin_unchecked(self: Pin<&mut Self>) -> Pin<&mut <T as Deref>::Target> {
        self.map_unchecked_mut(|s| s.0.deref_mut())
    }
}

impl<T: Deref> Deref for ArchivedRc<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Deref<Target = U>, U: PartialEq<V> + ?Sized, V: ?Sized> PartialEq<Rc<V>> for ArchivedRc<T> {
    fn eq(&self, other: &Rc<V>) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ArchiveRef + ?Sized> Archive for Rc<T> {
    type Archived = ArchivedRc<T::Reference>;
    type Resolver = RcResolver;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedRc(self.as_ref().resolve_ref(pos, resolver.0))
    }
}

impl<T: SerializeRef<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Rc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let key = Rc::into_raw(self.clone());
        unsafe { Rc::from_raw(key) };
        Ok(RcResolver(serializer.archive_shared(key as *const (), self.as_ref())?))
    }
}

impl<T: ArchiveRef + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Rc<T>, D> for Archived<Rc<T>>
where
    T::Reference: DeserializeRef<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Rc<T>, D::Error> {
        deserializer.deserialize_shared(
            &self.0,
            |ptr| Rc::<T>::from(unsafe { Box::from_raw(ptr) })
        )
    }
}

/// The resolver for [`Arc`].
pub struct ArcResolver(usize);

/// An archived [`Arc`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived. Multiple `ArchivedArc` may point to the same value.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedArc<T>(T);

impl<T: DerefMut> ArchivedArc<T> {
    /// Gets the value of this archived `Arc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedArc` pointers to the same value must not be
    /// dereferenced for the duration of the returned borrow.
    pub unsafe fn get_pin_unchecked(self: Pin<&mut Self>) -> Pin<&mut <T as Deref>::Target> {
        self.map_unchecked_mut(|s| s.0.deref_mut())
    }
}

impl<T: Deref> Deref for ArchivedArc<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: Deref<Target = U>, U: PartialEq<V> + ?Sized, V: ?Sized> PartialEq<Arc<V>> for ArchivedArc<T> {
    fn eq(&self, other: &Arc<V>) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ArchiveRef + ?Sized> Archive for Arc<T> {
    type Archived = ArchivedArc<T::Reference>;
    type Resolver = ArcResolver;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedArc(self.as_ref().resolve_ref(pos, resolver.0))
    }
}

impl<T: SerializeRef<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Arc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let key = Arc::into_raw(self.clone());
        unsafe { Arc::from_raw(key) };
        Ok(ArcResolver(serializer.archive_shared(key as *const (), self.as_ref())?))
    }
}

impl<T: ArchiveRef + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Arc<T>, D> for Archived<Arc<T>>
where
    T::Reference: DeserializeRef<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Arc<T>, D::Error> {
        deserializer.deserialize_shared(
            &self.0,
            |ptr| Arc::<T>::from(unsafe { Box::from_raw(ptr) })
        )
    }
}
