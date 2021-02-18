//! [`Archive`] implementation for shared pointers.

#[cfg(feature = "validation")]
pub mod validation;

use core::{cmp::PartialEq, mem, ops::Deref, pin::Pin};
use std::{rc::Rc, sync::Arc};
use crate::{Archive, ArchivePointee, ArchiveUnsized, Archived, Deserialize, DeserializeUnsized, RelPtr, Serialize, SerializeUnsized, de::{SharedDeserializer, SharedPointer}, ser::SharedSerializer};

impl<T: ?Sized> SharedPointer for Rc<T> {
    fn data_address(&self) -> *const () {
        Rc::as_ptr(self) as *const ()
    }
}

/// The resolver for [`Rc`].
pub struct RcResolver<T> {
    pos: usize,
    metadata_resolver: T,
}

/// An archived [`Rc`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived. Multiple `ArchivedRc` may point to the same value.
#[repr(transparent)]
pub struct ArchivedRc<T: ArchivePointee + ?Sized>(RelPtr<T>);

impl<T: ArchivePointee + ?Sized> ArchivedRc<T> {
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

impl<T: ArchivePointee + ?Sized> Deref for ArchivedRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<Rc<U>> for ArchivedRc<T> {
    fn eq(&self, other: &Rc<U>) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for Rc<T> {
    type Archived = ArchivedRc<T::Archived>;
    type Resolver = RcResolver<T::MetadataResolver>;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        unsafe { ArchivedRc(self.as_ref().resolve_unsized(pos, resolver.pos, resolver.metadata_resolver)) }
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Rc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(RcResolver {
            pos: serializer.archive_shared(&**self)?,
            metadata_resolver: self.deref().serialize_metadata(serializer)?,
        })
    }
}

impl<T: ArchiveUnsized + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Rc<T>, D> for Archived<Rc<T>>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Rc<T>, D::Error> {
        let raw_shared_ptr = deserializer.deserialize_shared::<T, Rc<T>, _>(
            &**self,
            |ptr| Rc::<T>::from(unsafe { Box::from_raw(ptr) }),
        )?;
        let shared_ptr = unsafe { Rc::<T>::from_raw(raw_shared_ptr) };
        mem::forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}

impl<T: ?Sized> SharedPointer for Arc<T> {
    fn data_address(&self) -> *const () {
        Arc::as_ptr(self) as *const ()
    }
}

/// The resolver for [`Arc`].
pub struct ArcResolver<T> {
    pos: usize,
    metadata_resolver: T,
}

/// An archived [`Arc`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived. Multiple `ArchivedArc` may point to the same value.
#[repr(transparent)]
pub struct ArchivedArc<T: ArchivePointee + ?Sized>(RelPtr<T>);

impl<T: ArchivePointee + ?Sized> ArchivedArc<T> {
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

impl<T: ArchivePointee + ?Sized> Deref for ArchivedArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<Arc<U>> for ArchivedArc<T> {
    fn eq(&self, other: &Arc<U>) -> bool {
        self.deref().eq(other.deref())
    }
}

impl<T: ArchiveUnsized + ?Sized> Archive for Arc<T> {
    type Archived = ArchivedArc<T::Archived>;
    type Resolver = ArcResolver<T::MetadataResolver>;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        unsafe { ArchivedArc(self.as_ref().resolve_unsized(pos, resolver.pos, resolver.metadata_resolver)) }
    }
}

impl<T: SerializeUnsized<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Arc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(ArcResolver {
            pos: serializer.archive_shared(&**self)?,
            metadata_resolver: self.deref().serialize_metadata(serializer)?,
        })
    }
}

impl<T: ArchiveUnsized + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Arc<T>, D> for Archived<Arc<T>>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Arc<T>, D::Error> {
        let raw_shared_ptr = deserializer.deserialize_shared(
            &**self,
            |ptr| Arc::<T>::from(unsafe { Box::from_raw(ptr) }),
        )?;
        let shared_ptr = unsafe { Arc::<T>::from_raw(raw_shared_ptr) };
        mem::forget(shared_ptr.clone());
        Ok(shared_ptr)
    }
}
