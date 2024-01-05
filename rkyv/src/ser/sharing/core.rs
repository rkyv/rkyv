use crate::ser::Sharing;

/// A shared pointer strategy that duplicates serializations of the same shared
/// pointer.
#[derive(Debug, Default)]
pub struct Duplicate;

impl<E> Sharing<E> for Duplicate {
    fn get_shared_ptr(&self, _: *const u8) -> Option<usize> {
        None
    }

    fn add_shared_ptr(&mut self, _: *const u8, _: usize) -> Result<(), E> {
        Ok(())
    }
}
