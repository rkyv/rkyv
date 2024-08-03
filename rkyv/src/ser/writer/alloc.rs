use crate::{
    alloc::vec::Vec,
    ser::{Positional, Writer},
    util::AlignedVec,
};

impl Positional for Vec<u8> {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl<E> Writer<E> for Vec<u8> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}

impl<const A: usize> Positional for AlignedVec<A> {
    #[inline]
    fn pos(&self) -> usize {
        self.len()
    }
}

impl<E, const A: usize> Writer<E> for AlignedVec<A> {
    fn write(&mut self, bytes: &[u8]) -> Result<(), E> {
        self.extend_from_slice(bytes);
        Ok(())
    }
}
