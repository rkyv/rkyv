use crate::de::pooling::{ErasedPtr, Pooling, PoolingState};

/// A shared pointer strategy that duplicates deserializations of the same
/// shared pointer.
#[derive(Debug, Default)]
pub struct Unpool;

impl<E> Pooling<E> for Unpool {
    fn start_pooling(&mut self, _: usize) -> PoolingState {
        PoolingState::Started
    }

    unsafe fn finish_pooling(
        &mut self,
        _: usize,
        _: ErasedPtr,
        _: unsafe fn(ErasedPtr),
    ) -> Result<(), E> {
        Ok(())
    }
}
