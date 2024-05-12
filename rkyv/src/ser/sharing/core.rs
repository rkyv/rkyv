use crate::ser::Sharing;

/// A shared pointer strategy that duplicates serializations of the same shared
/// pointer.
#[derive(Debug, Default)]
pub struct Unshare;

impl<E> Sharing<E> for Unshare {
    fn get_shared_ptr(&self, _: usize) -> Option<usize> {
        None
    }

    fn add_shared_ptr(&mut self, _: usize, _: usize) -> Result<(), E> {
        Ok(())
    }
}
