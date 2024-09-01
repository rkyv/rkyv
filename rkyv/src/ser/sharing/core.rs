use crate::ser::{sharing::SharingState, Sharing};

/// A shared pointer strategy that duplicates serializations of the same shared
/// pointer.
#[derive(Debug, Default)]
pub struct Unshare;

impl<E> Sharing<E> for Unshare {
    fn start_sharing(&mut self, _: usize) -> SharingState {
        SharingState::Started
    }

    fn finish_sharing(&mut self, _: usize, _: usize) -> Result<(), E> {
        Ok(())
    }
}
