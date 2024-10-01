use core::mem::size_of;

#[cfg(any(
    target_pointer_width = "64",
    target_arch = "aarch64",
    target_arch = "x86_64",
    target_arch = "wasm32",
))]
mod detail {
    pub type Word = u64;
    pub type NonZeroWord = core::num::NonZeroU64;
}
#[cfg(not(any(
    target_pointer_width = "64",
    target_arch = "aarch64",
    target_arch = "x86_64",
    target_arch = "wasm32",
)))]
mod detail {
    pub type Word = u32;
    pub type NonZeroWord = core::num::NonZeroU32;
}

use detail::*;

#[derive(Clone, Copy)]
pub struct Bitmask(Word);

impl Bitmask {
    pub const EMPTY: Self = Bitmask(0);

    #[inline]
    pub fn any_bit_set(self) -> bool {
        self.0 != 0
    }

    #[inline]
    pub fn remove_lowest_bit(self) -> Self {
        Self(self.0 & (self.0 - 1))
    }

    #[inline]
    pub fn lowest_set_bit(self) -> Option<usize> {
        let nonzero = NonZeroWord::new(self.0)?;
        Some(nonzero.trailing_zeros() as usize / 8)
    }
}

impl Iterator for Bitmask {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let bit = self.lowest_set_bit()?;
        *self = self.remove_lowest_bit();
        Some(bit)
    }
}

#[derive(Clone, Copy)]
pub struct Group(Word);

impl Group {
    pub const WIDTH: usize = size_of::<Word>();

    const fn repeat(byte: u8) -> Word {
        Word::from_ne_bytes([byte; Self::WIDTH])
    }

    /// # Safety
    ///
    /// `ptr` must be valid for reads and point to enough bytes for a `Word`.
    #[inline]
    pub unsafe fn read(ptr: *const u8) -> Self {
        // SAFETY: The caller has guaranteed that `ptr` is valid for reads and
        // points to enough bytes for a `Word`.
        #[cfg(target_endian = "little")]
        unsafe {
            Self(core::ptr::read_unaligned(ptr.cast()))
        }
        #[cfg(target_endian = "big")]
        unsafe {
            Self(core::ptr::read_unaligned(ptr.cast::<Word>()).swap_bytes())
        }
    }

    #[inline]
    pub fn match_byte(self, byte: u8) -> Bitmask {
        let zero_mask = self.0 ^ Self::repeat(byte);
        let bits = zero_mask.wrapping_sub(Self::repeat(0x01))
            & !zero_mask
            & Self::repeat(0x80);
        Bitmask(bits)
    }

    #[inline]
    pub fn match_empty(self) -> Bitmask {
        let bits = self.0 & Self::repeat(0x80);
        Bitmask(bits)
    }

    #[inline]
    pub fn match_full(self) -> Bitmask {
        let bits = !self.0 & Self::repeat(0x80);
        Bitmask(bits)
    }
}
