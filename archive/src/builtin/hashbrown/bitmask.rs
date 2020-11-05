use super::imp::{
    BitMaskWord,
    BITMASK_MASK,
    BITMASK_STRIDE,
};
#[cfg(feature = "nightly")]
use core::intrinsics;

#[derive(Copy, Clone)]
pub struct BitMask(pub BitMaskWord);

impl BitMask {
    #[inline]
    pub fn invert(self) -> Self {
        BitMask(self.0 ^ BITMASK_MASK)
    }

    #[inline]
    pub fn remove_lowest_bit(self) -> Self {
        BitMask(self.0 & (self.0 - 1))
    }

    #[inline]
    pub fn any_bit_set(self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn lowest_set_bit(self) -> Option<usize> {
        if self.0 == 0 {
            None
        } else {
            Some(unsafe { self.lowest_set_bit_nonzero() })
        }
    }

    #[inline]
    #[cfg(feature = "nightly")]
    pub unsafe fn lowest_set_bit_nonzero(self) -> usize {
        intrinsics::cttz_nonzero(self.0) as usize / BITMASK_STRIDE
    }
    #[inline]
    #[cfg(not(feature = "nightly"))]
    pub unsafe fn lowest_set_bit_nonzero(self) -> usize {
        self.trailing_zeros()
    }

    #[inline]
    #[cfg(not(feature = "nightly"))]
    pub fn trailing_zeros(self) -> usize {
        if cfg!(target_arch = "arm") && BITMASK_STRIDE % 8 == 0 {
            self.0.swap_bytes().leading_zeros() as usize / BITMASK_STRIDE
        } else {
            self.0.trailing_zeros() as usize / BITMASK_STRIDE
        }
    }
}

impl IntoIterator for BitMask {
    type Item = usize;
    type IntoIter = BitMaskIter;

    #[inline]
    fn into_iter(self) -> BitMaskIter {
        BitMaskIter(self)
    }
}

pub struct BitMaskIter(BitMask);

impl Iterator for BitMaskIter {
    type Item = usize;

    #[inline]
    fn next(&mut self) -> Option<usize> {
        let bit = self.0.lowest_set_bit()?;
        self.0 = self.0.remove_lowest_bit();
        Some(bit)
    }
}