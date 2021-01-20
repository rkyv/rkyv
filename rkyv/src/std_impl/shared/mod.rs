#[cfg(feature = "validation")]
pub mod validation;

use core::{alloc, ops::{Deref, DerefMut}, pin::Pin};
use std::{any::Any, collections::HashMap, rc::Rc, sync::Arc};
use crate::{Archive, Archived, ArchiveRef, Deserialize, Deserializer, DeserializeRef, Fallible, Reference, Serialize, SerializeRef, Serializer};

pub trait SharedSerializer: Serializer {
    fn serialize_shared_ref<T: ArchiveRef + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error>
    where
        T: SerializeRef<Self>;
}

/// A wrapper around a serializer that adds support for [`SharedSerializer`].
pub struct SharedSerializerAdapter<S> {
    inner: S,
    shared_resolvers: HashMap<*const u8, usize>,
}

impl<S> SharedSerializerAdapter<S> {
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            shared_resolvers: HashMap::new(),
        }
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Fallible> Fallible for SharedSerializerAdapter<S> {
    type Error = S::Error;
}

impl<S: Serializer> Serializer for SharedSerializerAdapter<S> {
    fn pos(&self) -> usize {
        self.inner.pos()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(bytes)
    }

    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        self.inner.pad(padding)
    }

    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        self.inner.align(align)
    }

    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.inner.align_for::<T>()
    }

    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        self.inner.resolve_aligned(value, resolver)
    }
}

impl<S: Serializer> SharedSerializer for SharedSerializerAdapter<S> {
    fn serialize_shared_ref<T: SerializeRef<Self> + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let key = (value as *const T).cast::<u8>();
        if let Some(existing) = self.shared_resolvers.get(&key) {
            Ok(existing.clone())
        } else {
            let resolver = value.serialize_ref(self)?;
            self.shared_resolvers.insert(key, resolver);
            Ok(resolver)
        }
    }
}

pub trait SharedDeserializer: Deserializer {
    fn deserialize_shared_ref<T: ArchiveRef + ?Sized, P: Clone + 'static>(&mut self, reference: &T::Reference, to_shared: impl FnOnce(*mut T) -> P) -> Result<P, Self::Error>
    where
        T::Reference: DeserializeRef<T, Self>;
}

pub struct SharedDeserializerAdapter<D> {
    inner: D,
    shared_pointers: HashMap<*const (), Box<dyn Any>>,
}

impl<D> SharedDeserializerAdapter<D> {
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            shared_pointers: HashMap::new(),
        }
    }

    pub fn into_inner(self) -> D {
        self.inner
    }
}

impl<D: Deserializer> Fallible for SharedDeserializerAdapter<D> {
    type Error = D::Error;
}

impl<D: Deserializer> Deserializer for SharedDeserializerAdapter<D> {
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error> {
        self.inner.alloc(layout)
    }
}

impl<D: Deserializer> SharedDeserializer for SharedDeserializerAdapter<D> {
    fn deserialize_shared_ref<T: ArchiveRef + ?Sized, P: Clone + 'static>(&mut self, reference: &T::Reference, to_shared: impl FnOnce(*mut T) -> P) -> Result<P, Self::Error>
    where
        T::Reference: DeserializeRef<T, Self>,
    {
        let archived_ptr = &**reference as *const T::Archived;

        // This is safe to read a single pointer from because it's at least a thin pointer
        let data_ptr = unsafe { (&archived_ptr as *const *const T::Archived).cast::<*const ()>().read() };

        let shared_ptr = if let Some(shared_ptr) = self.shared_pointers.get(&data_ptr) {
            // TODO: custom error type with variant for downcast failure
            (**shared_ptr).downcast_ref::<P>().unwrap().clone()
        } else {
            let deserialized = unsafe { reference.deserialize_ref(self)? };
            let shared_ptr = to_shared(deserialized);
            self.shared_pointers.insert(data_ptr, Box::new(shared_ptr.clone()));
            shared_ptr
        };
        Ok(shared_ptr)
    }
}

/// The resolver for `Rc`.
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
        Ok(RcResolver(serializer.serialize_shared_ref(self.as_ref())?))
    }
}

impl<T: ArchiveRef + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Rc<T>, D> for Archived<Rc<T>>
where
    Reference<T>: DeserializeRef<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Rc<T>, D::Error> {
        deserializer.deserialize_shared_ref(
            &self.0,
            |ptr| Rc::<T>::from(unsafe { Box::from_raw(ptr) })
        )
    }
}

/// The resolver for `Arc`.
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
        Ok(ArcResolver(serializer.serialize_shared_ref(self.as_ref())?))
    }
}

impl<T: ArchiveRef + ?Sized + 'static, D: SharedDeserializer + ?Sized> Deserialize<Arc<T>, D> for Archived<Arc<T>>
where
    Reference<T>: DeserializeRef<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Arc<T>, D::Error> {
        deserializer.deserialize_shared_ref(
            &self.0,
            |ptr| Arc::<T>::from(unsafe { Box::from_raw(ptr) })
        )
    }
}
