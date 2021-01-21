use std::collections::HashMap;
use crate::{
    ser::{Serializer, SharedSerializer},
    Archive,
    Fallible,
    SerializeRef,
};

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
    fn archive_shared<T: SerializeRef<Self> + ?Sized>(&mut self, value: &T) -> Result<usize, Self::Error> {
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