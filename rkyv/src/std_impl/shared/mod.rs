//! [`Archive`] implementation for shared pointers.

#[cfg(feature = "validation")]
pub mod validation;

use core::{cmp::PartialEq, ops::Deref, pin::Pin};
use std::{rc::Rc, sync::Arc};
use crate::{
    de::SharedDeserializer,
    ser::SharedSerializer,
    Archive,
    Archived,
    ArchivePtr,
    ArchiveUnsized,
    Deserialize,
    DeserializeUnsized,
    RelPtr,
    Serialize,
    SerializeUnsized,
};

/// The resolver for [`Rc`].
pub struct RcResolver(usize);

/// An archived [`Rc`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived. Multiple `ArchivedRc` may point to the same value.
#[repr(transparent)]
pub struct ArchivedRc<T: ArchivePtr + ?Sized>(RelPtr<T>);

impl<T: ArchivePtr + ?Sized> ArchivedRc<T> {
    /// Gets the value of this archived `Rc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` pointers to the same value must not be
    /// dereferenced for the duration of the returned borrow.
    pub unsafe fn get_pin_unchecked(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.map_unchecked_mut(|s| &mut *s.0.as_mut_ptr())
    }
}

impl<T: ArchivePtr + ?Sized> Deref for ArchivedRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T: ArchivePtr + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<Rc<U>> for ArchivedRc<T> {
    fn eq(&self, other: &Rc<U>) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for Rc<T> {
    type Archived = ArchivedRc<T::Archived>;
    type Resolver = RcResolver;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedRc(self.as_ref().resolve_unsized(pos, resolver.0))
    }
}

impl<T: ArchiveUnsized + SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Rc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(RcResolver(serializer.archive_shared(&**self)?))
    }
}

impl<T: ArchiveUnsized + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Rc<T>, D> for Archived<Rc<T>>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Rc<T>, D::Error> {
        deserializer.deserialize_shared(
            &**self,
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
#[repr(transparent)]
pub struct ArchivedArc<T: ArchivePtr + ?Sized>(RelPtr<T>);

impl<T: ArchivePtr + ?Sized> ArchivedArc<T> {
    /// Gets the value of this archived `Arc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedArc` pointers to the same value must not be
    /// dereferenced for the duration of the returned borrow.
    pub unsafe fn get_pin_unchecked(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.map_unchecked_mut(|s| &mut *s.0.as_mut_ptr())
    }
}

impl<T: ArchivePtr + ?Sized> Deref for ArchivedArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T: ArchivePtr + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<Arc<U>> for ArchivedArc<T> {
    fn eq(&self, other: &Arc<U>) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for Arc<T> {
    type Archived = ArchivedArc<T::Archived>;
    type Resolver = ArcResolver;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedArc(self.as_ref().resolve_unsized(pos, resolver.0))
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Arc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(ArcResolver(serializer.archive_shared(&**self)?))
    }
}

impl<T: ArchiveUnsized + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Arc<T>, D> for Archived<Arc<T>>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Arc<T>, D::Error> {
        deserializer.deserialize_shared(
            &**self,
            |ptr| Arc::<T>::from(unsafe { Box::from_raw(ptr) })
        )
    }
}
