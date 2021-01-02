use super::{bitmask::BitMask, EMPTY};
use core::{mem, ptr};

#[cfg(any(
    target_pointer_width = "64",
    target_arch = "aarch64",
    target_arch = "x86_64"
))]
type GroupWord = u64;

#[cfg(all(
    target_pointer_width = "32",
    not(target_arch = "aarch64"),
    not(target_arch = "x86_64")
))]
type GroupWord = u32;

pub type BitMaskWord = GroupWord;
pub const BITMASK_STRIDE: usize = 8;
pub const BITMASK_MASK: BitMaskWord = 0x8080_8080_8080_8080_u64 as GroupWord;

#[inline]
fn repeat(byte: u8) -> GroupWord {
    GroupWord::from_ne_bytes([byte; Group::WIDTH])
}

#[derive(Copy, Clone)]
pub struct Group(GroupWord);

impl Group {
    pub const WIDTH: usize = mem::size_of::<Self>();

    #[inline]
    pub fn static_empty() -> &'static [u8] {
        union AlignedBytes {
            _align: Group,
            bytes: [u8; Group::WIDTH],
        }
        static ALIGNED_BYTES: AlignedBytes = AlignedBytes {
            bytes: [EMPTY; Group::WIDTH],
        };
        unsafe { &ALIGNED_BYTES.bytes }
    }

    #[inline]
    pub unsafe fn load(ptr: *const u8) -> Self {
        Group(ptr::read_unaligned(ptr as *const _))
    }

    #[inline]
    pub unsafe fn load_aligned(ptr: *const u8) -> Self {
        debug_assert_eq!(ptr as usize & (mem::align_of::<Self>() - 1), 0);
        Group(ptr::read(ptr as *const _))
    }

    #[inline]
    pub fn match_byte(self, byte: u8) -> BitMask {
        let cmp = self.0 ^ repeat(byte);
        BitMask((cmp.wrapping_sub(repeat(0x01)) & !cmp & repeat(0x80)).to_le())
    }

    #[inline]
    pub fn match_empty(self) -> BitMask {
        BitMask((self.0 & (self.0 << 1) & repeat(0x80)).to_le())
    }

    #[inline]
    pub fn match_full(self) -> BitMask {
        self.match_empty().invert()
    }
}
