use crate::de::{Pooling, SharedPointer};

/// A shared pointer strategy that duplicates deserializations of the same
/// shared pointer.
#[derive(Debug, Default)]
pub struct Duplicate;

impl<E> Pooling<E> for Duplicate {
    fn get_shared_ptr(&mut self, _: *const u8) -> Option<&dyn SharedPointer> {
        None
    }

    fn add_shared_ptr(
        &mut self,
        _: *const u8,
        _: Box<dyn SharedPointer>,
    ) -> Result<(), E> {
        Ok(())
    }
}
