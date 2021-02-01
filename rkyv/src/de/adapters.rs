//! Adapters wrap deserializers and add support for deserializer traits.

use core::{alloc, any::Any, fmt};
use std::{collections::HashMap, error::Error};
use crate::{
    de::{Deserializer, SharedDeserializer},
    ArchiveUnsized,
    DeserializeUnsized,
    Fallible,
};

#[derive(Debug)]
pub enum SharedDeserializerError<E> {
    MismatchedPointerTypeError,
    InnerError(E),
}

impl<E: fmt::Display> fmt::Display for SharedDeserializerError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SharedDeserializerError::MismatchedPointerTypeError => write!(f, "mismatched shared pointer types"),
            SharedDeserializerError::InnerError(e) => write!(f, "{}", e),
        }
    }
}

impl<E: Error + 'static> Error for SharedDeserializerError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SharedDeserializerError::MismatchedPointerTypeError => None,
            SharedDeserializerError::InnerError(e) => Some(e as &(dyn Error + 'static)),
        }
    }
}

/// An adapter that adds shared deserialization support to a deserializer.
pub struct SharedDeserializerAdapter<D> {
    inner: D,
    shared_pointers: HashMap<*const (), Box<dyn Any>>,
}

impl<D> SharedDeserializerAdapter<D> {
    /// Wraps the given deserializer and adds shared memory support.
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            shared_pointers: HashMap::new(),
        }
    }

    /// Consumes the adapter and returns the underlying deserializer.
    pub fn into_inner(self) -> D {
        self.inner
    }
}

impl<D: Deserializer> Fallible for SharedDeserializerAdapter<D> {
    type Error = SharedDeserializerError<D::Error>;
}

impl<D: Deserializer> Deserializer for SharedDeserializerAdapter<D> {
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error> {
        Ok(self.inner.alloc(layout).map_err(SharedDeserializerError::InnerError)?)
    }
}

impl<D: Deserializer> SharedDeserializer for SharedDeserializerAdapter<D> {
    fn deserialize_shared<T: ArchiveUnsized + ?Sized, P: Clone + 'static>(&mut self, value: &T::Archived, to_shared: impl FnOnce(*mut T) -> P) -> Result<P, Self::Error>
    where
        T::Archived: DeserializeUnsized<T, Self>,
    {
        let key = value as *const T::Archived as *const ();
        let shared_ptr = if let Some(shared_ptr) = self.shared_pointers.get(&key) {
            (**shared_ptr).downcast_ref::<P>().ok_or(SharedDeserializerError::MismatchedPointerTypeError)?.clone()
        } else {
            let deserialized = unsafe { value.deserialize_unsized(self)? };
            let shared_ptr = to_shared(deserialized);
            self.shared_pointers.insert(key, Box::new(shared_ptr.clone()));
            shared_ptr
        };
        Ok(shared_ptr)
    }
}
