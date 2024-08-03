#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86;
use core::{mem::size_of, num::NonZeroU16};

type Word = x86::__m128i;

#[derive(Clone, Copy)]
pub struct Bitmask(u16);

impl Bitmask {
    pub const EMPTY: Self = Self(0);

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
        let nonzero = NonZeroU16::new(self.0)?;
        Some(nonzero.trailing_zeros() as usize)
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

    /// # Safety
    ///
    /// `ptr` must be valid for reads and point to enough bytes for a `Word`.
    #[inline]
    pub unsafe fn read(ptr: *const u8) -> Self {
        // SAFETY: The caller has guaranteed that `ptr` is valid for reads and
        // points to enough bytes for a `Word`.
        unsafe { Self(x86::_mm_loadu_si128(ptr.cast())) }
    }

    #[inline]
    pub fn match_byte(self, byte: u8) -> Bitmask {
        unsafe {
            let cmp =
                x86::_mm_cmpeq_epi8(self.0, x86::_mm_set1_epi8(byte as i8));
            Bitmask(x86::_mm_movemask_epi8(cmp) as u16)
        }
    }

    #[inline]
    pub fn match_empty(self) -> Bitmask {
        unsafe { Bitmask(x86::_mm_movemask_epi8(self.0) as u16) }
    }

    #[inline]
    pub fn match_full(self) -> Bitmask {
        unsafe { Bitmask(!x86::_mm_movemask_epi8(self.0) as u16) }
    }
}
