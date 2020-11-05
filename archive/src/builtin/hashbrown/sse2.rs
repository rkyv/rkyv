use core::mem;
use super::{
    bitmask::BitMask,
    EMPTY,
};

#[cfg(target_arch = "x86")]
use core::arch::x86;
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64 as x86;

pub type BitMaskWord = u16;
pub const BITMASK_STRIDE: usize = 1;
pub const BITMASK_MASK: BitMaskWord = 0xFFFF;

#[derive(Copy, Clone)]
pub struct Group(x86::__m128i);

impl Group {
    pub const WIDTH: usize = mem::size_of::<Self>();

    pub fn static_empty() -> &'static [u8] {
        union AlignedBytes {
            _align: Group,
            bytes: [u8; Group::WIDTH],
        };
        static ALIGNED_BYTES: AlignedBytes = AlignedBytes {
            bytes: [EMPTY; Group::WIDTH],
        };
        unsafe { &ALIGNED_BYTES.bytes }
    }

    #[inline]
    pub unsafe fn load(ptr: *const u8) -> Self {
        Group(x86::_mm_loadu_si128(ptr as *const _))
    }

    #[inline]
    pub unsafe fn load_aligned(ptr: *const u8) -> Self {
        debug_assert_eq!(ptr as usize & (mem::align_of::<Self>() - 1), 0);
        Group(x86::_mm_load_si128(ptr as *const _))
    }

    #[inline]
    pub fn match_byte(self, byte: u8) -> BitMask {
        unsafe {
            let cmp = x86::_mm_cmpeq_epi8(self.0, x86::_mm_set1_epi8(byte as i8));
            BitMask(x86::_mm_movemask_epi8(cmp) as u16)
        }
    }

    #[inline]
    pub fn match_empty(self) -> BitMask {
        self.match_byte(EMPTY)
    }

    #[inline]
    pub fn match_full(&self) -> BitMask {
        self.match_empty().invert()
    }
}