use super::{Pooling, SharedPointer};

/// A shared pointer strategy that duplicates deserializations of the same
/// shared pointer.
#[derive(Debug, Default)]
pub struct Duplicate;

impl<E> Pooling<E> for Duplicate {
    fn get_shared_ptr(&mut self, _: usize) -> Option<&dyn SharedPointer> {
        None
    }

    fn add_shared_ptr(
        &mut self,
        _: usize,
        _: Box<dyn SharedPointer>,
    ) -> Result<(), E> {
        Ok(())
    }
}
