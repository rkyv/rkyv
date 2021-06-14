//! Adapters wrap serializers and add support for serializer traits.

use crate::{
    ser::{Serializer, SharedSerializer},
    Archive, Fallible, SerializeUnsized,
};
#[cfg(all(feature = "alloc", not(feature = "std")))]
use hashbrown::HashMap;
#[cfg(feature = "std")]
use std::collections::HashMap;

/// An adapter that adds shared serialization support to a serializer.
pub struct SharedSerializerAdapter<S> {
    inner: S,
    shared_resolvers: HashMap<*const u8, usize>,
}

impl<S> SharedSerializerAdapter<S> {
    /// Wraps the given serializer and adds shared memory support.
    #[inline]
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            shared_resolvers: HashMap::new(),
        }
    }

    /// Consumes the adapter and returns the underlying serializer.
    #[inline]
    pub fn into_inner(self) -> S {
        self.inner
    }
}

impl<S: Fallible> Fallible for SharedSerializerAdapter<S> {
    type Error = S::Error;
}

impl<S: Serializer> Serializer for SharedSerializerAdapter<S> {
    #[inline]
    fn pos(&self) -> usize {
        self.inner.pos()
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner.write(bytes)
    }

    #[inline]
    fn pad(&mut self, padding: usize) -> Result<(), Self::Error> {
        self.inner.pad(padding)
    }

    #[inline]
    fn align(&mut self, align: usize) -> Result<usize, Self::Error> {
        self.inner.align(align)
    }

    #[inline]
    fn align_for<T>(&mut self) -> Result<usize, Self::Error> {
        self.inner.align_for::<T>()
    }

    #[inline]
    unsafe fn resolve_aligned<T: Archive + ?Sized>(
        &mut self,
        value: &T,
        resolver: T::Resolver,
    ) -> Result<usize, Self::Error> {
        self.inner.resolve_aligned(value, resolver)
    }
}

impl<S: Serializer> SharedSerializer for SharedSerializerAdapter<S> {
    fn serialize_shared<T: SerializeUnsized<Self> + ?Sized>(
        &mut self,
        value: &T,
    ) -> Result<usize, Self::Error> {
        let key = (value as *const T).cast();
        if let Some(existing) = self.shared_resolvers.get(&key) {
            Ok(*existing)
        } else {
            let resolver = value.serialize_unsized(self)?;
            self.shared_resolvers.insert(key, resolver);
            Ok(resolver)
        }
    }
}
