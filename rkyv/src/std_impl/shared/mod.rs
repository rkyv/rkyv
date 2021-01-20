#[cfg(feature = "validation")]
pub mod validation;

use core::{any::{Any, TypeId}, fmt, ops::{Deref, DerefMut}, pin::Pin};
use std::{collections::HashMap, error::Error, rc::Rc, sync::Arc};
use crate::{Archive, ArchiveRef, Fallible, Serialize, SerializeRef, Serializer, SharedSerializer};

#[derive(Debug)]
pub enum SharedSerializerAdapterError<T> {
    Inner(T),
    ResolverTypeMismatch {
        expected: TypeId,
        found: TypeId,
    },
}

impl<T: fmt::Display> fmt::Display for SharedSerializerAdapterError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedSerializerAdapterError::Inner(e) => write!(f, "inner error: {}", e),
            SharedSerializerAdapterError::ResolverTypeMismatch { expected, found } => write!(f, "shared value requested as `{:?}` but previously serialized as `{:?}`", expected, found),
        }
    }
}

impl<E: Error + 'static> Error for SharedSerializerAdapterError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedSerializerAdapterError::Inner(e) => Some(e as &dyn Error),
            SharedSerializerAdapterError::ResolverTypeMismatch { .. } => None,
        }
    }
}

/// A wrapper around a serializer that adds support for [`SharedWrite`].
pub struct SharedSerializerAdapter<S> {
    inner: S,
    shared_resolvers: HashMap<*const u8, (TypeId, usize)>,
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
    type Error = SharedSerializerAdapterError<S::Error>;
}

impl<S: Serializer> Serializer for SharedSerializerAdapter<S> {
    fn pos(&self) -> usize {
        self.inner.pos()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(bytes).map_err(SharedSerializerAdapterError::Inner)
    }

    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        self.inner.pad(padding).map_err(SharedSerializerAdapterError::Inner)
    }

    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        self.inner.align(align).map_err(SharedSerializerAdapterError::Inner)
    }

    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.inner.align_for::<T>().map_err(SharedSerializerAdapterError::Inner)
    }

    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        self.inner.resolve_aligned(value, resolver).map_err(SharedSerializerAdapterError::Inner)
    }
}

impl<S: Serializer> SharedSerializer for SharedSerializerAdapter<S> {
    fn serialize_shared_ref<T: SerializeRef<Self> + ?Sized + 'static>(&mut self, value: &T) -> Result<usize, Self::Error> {
        let key = (value as *const T).cast::<u8>();
        let type_id = value.type_id();
        if let Some((existing_type_id, existing)) = self.shared_resolvers.get(&key) {
            if existing_type_id == &type_id {
                Ok(existing.clone())
            } else {
                Err(SharedSerializerAdapterError::ResolverTypeMismatch {
                    expected: type_id,
                    found: *existing_type_id,
                })
            }
        } else {
            let resolver = value.serialize_ref(self)?;
            self.shared_resolvers.insert(key, (type_id, resolver));
            Ok(resolver)
        }
    }
}

/// An archived [`Rc`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived. Multiple `ArchivedRc` may point to the same value.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedRc<T>(T);

impl<T: DerefMut> ArchivedRc<T> {
    /// Gets the value of this archived `Rc` or `Arc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` pointers to the same value must not be
    /// dereferenced for the duration of the returned borrow.
    pub fn get_pin_unchecked(self: Pin<&mut Self>) -> Pin<&mut <T as Deref>::Target> {
        unsafe { self.map_unchecked_mut(|s| s.0.deref_mut()) }
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

impl<T: Deref<Target = U>, U: PartialEq<V> + ?Sized, V: ?Sized> PartialEq<Arc<V>> for ArchivedRc<T> {
    fn eq(&self, other: &Arc<V>) -> bool {
        self.deref().eq(other.deref())
    }
}

/// The resolver for `Rc` and `Arc`.
pub struct RcResolver(usize);

impl<T: ArchiveRef + ?Sized> Archive for Rc<T> {
    type Archived = ArchivedRc<T::Reference>;
    type Resolver = RcResolver;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedRc(self.as_ref().resolve_ref(pos, resolver.0))
    }
}

impl<T: ArchiveRef + ?Sized> Archive for Arc<T> {
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

impl<T: SerializeRef<S> + ?Sized + 'static, S: SharedSerializer + ?Sized> Serialize<S> for Arc<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(RcResolver(serializer.serialize_shared_ref(self.as_ref())?))
    }
}
