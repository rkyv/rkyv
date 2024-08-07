use core::{arch::aarch64, mem::size_of, num::NonZeroU64};

type Word = aarch64::uint8x16_t;

#[derive(Clone, Copy)]
pub struct Bitmask(u64);

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
        let nonzero = NonZeroU64::new(self.0)?;
        Some(nonzero.trailing_zeros() as usize / 4)
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
        unsafe { Self(aarch64::vld1q_u8(ptr)) }
    }

    #[inline]
    fn unpack(cmp: Word) -> Bitmask {
        // 0xFF_FF_FF_00_00_FF_00_00 => 0xFF_F0_0F_00
        let nibbles = unsafe {
            aarch64::vshrn_n_u16(aarch64::vreinterpretq_u16_u8(cmp), 4)
        };
        // 0xFF_F0_0F_00 => 0x88_80_08_00
        let bits =
            unsafe { aarch64::vand_u8(nibbles, aarch64::vdup_n_u8(0x88)) };
        // 0x88_80_08_00 => 0x88800800
        let result = unsafe {
            aarch64::vget_lane_u64(aarch64::vreinterpret_u64_u8(bits), 0)
        };
        Bitmask(result)
    }

    #[inline]
    pub fn match_byte(self, byte: u8) -> Bitmask {
        unsafe {
            Self::unpack(aarch64::vceqq_u8(self.0, aarch64::vdupq_n_u8(byte)))
        }
    }

    #[inline]
    pub fn match_empty(self) -> Bitmask {
        unsafe {
            Self::unpack(aarch64::vcltzq_s8(aarch64::vreinterpretq_s8_u8(
                self.0,
            )))
        }
    }

    #[inline]
    pub fn match_full(self) -> Bitmask {
        unsafe {
            Self::unpack(aarch64::vcgezq_s8(aarch64::vreinterpretq_s8_u8(
                self.0,
            )))
        }
    }
}
