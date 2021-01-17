use core::{any::{Any, TypeId}, fmt, ops::{Deref, DerefMut}, pin::Pin};
use std::{collections::HashMap, error::Error, rc::Rc};
use crate::{Archive, ArchiveRef, ReferenceResolver, Resolve, Serialize, SerializeRef, SharedWrite, Write};

#[derive(Debug)]
pub enum SharedWriterError<T> {
    Inner(T),
    ResolverTypeMismatch {
        expected: TypeId,
        found: TypeId,
    },
}

impl<T: fmt::Display> fmt::Display for SharedWriterError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedWriterError::Inner(e) => write!(f, "inner error: {}", e),
            SharedWriterError::ResolverTypeMismatch { expected, found } => write!(f, "shared value requested as `{:?}` but previously serialized as `{:?}`", expected, found),
        }
    }
}

impl<E: Error + 'static> Error for SharedWriterError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedWriterError::Inner(e) => Some(e as &dyn Error),
            SharedWriterError::ResolverTypeMismatch { .. } => None,
        }
    }
}

/// A wrapper around a writer that adds support for [`SharedWrite`].
pub struct SharedWriter<W: Write> {
    inner: W,
    shared_resolvers: HashMap<*const (), Box<dyn Any>>,
}

impl<W: Write> Write for SharedWriter<W> {
    type Error = SharedWriterError<W::Error>;

    fn pos(&self) -> usize {
        self.inner.pos()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(bytes).map_err(SharedWriterError::Inner)
    }

    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        self.inner.pad(padding).map_err(SharedWriterError::Inner)
    }

    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        self.inner.align(align).map_err(SharedWriterError::Inner)
    }

    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.inner.align_for::<T>().map_err(SharedWriterError::Inner)
    }

    unsafe fn resolve_aligned<T: ?Sized, R: Resolve<T>>(
        &mut self,
        value: &T,
        resolver: R,
    ) -> Result<usize, Self::Error> {
        self.inner.resolve_aligned(value, resolver).map_err(SharedWriterError::Inner)
    }
}

impl<W: Write> SharedWrite for SharedWriter<W> {
    fn serialize_shared_ref<T: SerializeRef<Self> + ?Sized + 'static>(&mut self, value: &T) -> Result<ReferenceResolver<T>, Self::Error>
    where
        ReferenceResolver<T>: Clone + 'static
    {
        let key = (value as *const T).cast::<()>();
        if let Some(existing) = self.shared_resolvers.get(&key) {
            if let Some(resolver) = existing.downcast_ref::<T::ReferenceResolver>() {
                Ok(resolver.clone())
            } else {
                Err(SharedWriterError::ResolverTypeMismatch {
                    expected: value.type_id(),
                    found: existing.type_id(),
                })
            }
        } else {
            let resolver = value.serialize_ref(self)?;
            self.shared_resolvers.insert(key, Box::new(resolver.clone()) as Box<dyn Any>);
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
    /// Gets the value of this archived Rc.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` or `ArchivedWeak` pointers to the same value must
    /// not be dereferenced for the duration of the returned borrow.
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

/// The resolver for `Rc`.
#[derive(Clone)]
pub struct RcResolver<T>(T);

impl<T: ArchiveRef + ?Sized> Resolve<Rc<T>> for RcResolver<T::ReferenceResolver> {
    type Archived = ArchivedRc<T::Reference>;

    fn resolve(self, pos: usize, value: &Rc<T>) -> Self::Archived {
        ArchivedRc(self.0.resolve(pos, value.as_ref()))
    }
}

impl<T: ArchiveRef + ?Sized> Archive for Rc<T> {
    type Archived = ArchivedRc<T::Reference>;
    type Resolver = RcResolver<T::ReferenceResolver>;
}

impl<T: SerializeRef<W> + ?Sized + 'static, W: SharedWrite + ?Sized> Serialize<W> for Rc<T>
where
    T::ReferenceResolver: Any + Clone,
{
    fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(RcResolver(writer.serialize_shared_ref(self.as_ref())?))
    }
}