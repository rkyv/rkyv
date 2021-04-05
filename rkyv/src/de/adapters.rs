//! Adapters wrap deserializers and add support for deserializer traits.

use crate::{
    de::{Deserializer, SharedDeserializer, SharedPointer},
    ArchiveUnsized, DeserializeUnsized, Fallible,
};
use core::alloc;
use std::collections::HashMap;

/// An adapter that adds shared deserialization support to a deserializer.
pub struct SharedDeserializerAdapter<D> {
    inner: D,
    shared_pointers: HashMap<*const (), Box<dyn SharedPointer>>,
}

impl<D> SharedDeserializerAdapter<D> {
    /// Wraps the given deserializer and adds shared memory support.
    #[inline]
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            shared_pointers: HashMap::new(),
        }
    }

    /// Consumes the adapter and returns the underlying deserializer.
    #[inline]
    pub fn into_inner(self) -> D {
        self.inner
    }
}

impl<D: Deserializer> Fallible for SharedDeserializerAdapter<D> {
    type Error = D::Error;
}

impl<D: Deserializer> Deserializer for SharedDeserializerAdapter<D> {
    #[inline]
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error> {
        self.inner.alloc(layout)
    }
}

impl<D: Deserializer> SharedDeserializer for SharedDeserializerAdapter<D> {
    fn deserialize_shared<
        T: ArchiveUnsized + ?Sized,
        P: SharedPointer + 'static,
        F: FnOnce(*mut T) -> P,
    >(
        &mut self,
        value: &T::Archived,
        to_shared: F,
    ) -> Result<*const T, Self::Error>
    where
        T::Archived: DeserializeUnsized<T, Self>,
    {
        let key = value as *const T::Archived as *const ();
        let metadata = T::Archived::deserialize_metadata(value, self)?;

        if let Some(shared_pointer) = self.shared_pointers.get(&key) {
            Ok(ptr_meta::from_raw_parts(
                shared_pointer.data_address() as *const (),
                metadata,
            ))
        } else {
            let deserialized_data = unsafe { value.deserialize_unsized(self)? };
            let shared_ptr = to_shared(ptr_meta::from_raw_parts_mut(deserialized_data, metadata));
            let data_address = shared_ptr.data_address();

            self.shared_pointers
                .insert(key, Box::new(shared_ptr) as Box<dyn SharedPointer>);
            Ok(ptr_meta::from_raw_parts(data_address, metadata))
        }
    }
}
