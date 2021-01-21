use core::{alloc, any::Any};
use std::collections::HashMap;
use crate::{
    de::{Deserializer, SharedDeserializer},
    ArchiveRef,
    DeserializeRef,
    Fallible,
};

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
    fn deserialize_shared<T: ArchiveRef + ?Sized, P: Clone + 'static>(&mut self, reference: &T::Reference, to_shared: impl FnOnce(*mut T) -> P) -> Result<P, Self::Error>
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