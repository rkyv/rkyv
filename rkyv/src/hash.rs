//! Hashing support for archived hash maps and sets.

use core::{
    hash::{Hash, Hasher},
    ops::BitXor as _,
};

use crate::primitive::{FixedIsize, FixedUsize};

/// A cross-platform 64-bit implementation of fxhash.
#[derive(Default)]
pub struct FxHasher64 {
    hash: u64,
}

#[inline]
fn hash_word(hash: u64, word: u64) -> u64 {
    const ROTATE: u32 = 5;
    const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

    hash.rotate_left(ROTATE).bitxor(word).wrapping_mul(SEED)
}

#[inline]
fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    let ptr = bytes.as_ptr();
    let len = bytes.len();

    for i in 0..len / 8 {
        let bytes = unsafe { ptr.cast::<[u8; 8]>().add(i).read_unaligned() };
        hash = hash_word(hash, u64::from_le_bytes(bytes));
    }

    if bytes.len() & 4 != 0 {
        let bytes = unsafe {
            ptr.add(bytes.len() & !7).cast::<[u8; 4]>().read_unaligned()
        };
        hash = hash_word(hash, u32::from_le_bytes(bytes).into());
    }

    if bytes.len() & 2 != 0 {
        let bytes = unsafe {
            ptr.add(bytes.len() & !3).cast::<[u8; 2]>().read_unaligned()
        };
        hash = hash_word(hash, u16::from_le_bytes(bytes).into());
    }

    if bytes.len() & 1 != 0 {
        let byte = unsafe { ptr.add(len - 1).read() };
        hash = hash_word(hash, byte.into());
    }

    hash
}

impl Hasher for FxHasher64 {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.hash = hash_bytes(self.hash, bytes);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }

    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.hash = hash_word(self.hash, i as u64);
    }

    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.hash = hash_word(self.hash, i as u64);
    }

    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.hash = hash_word(self.hash, i as u64);
    }

    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.hash = hash_word(self.hash, i);
    }

    #[inline]
    fn write_u128(&mut self, i: u128) {
        let bytes = i.to_ne_bytes();
        let ptr = bytes.as_ptr().cast::<[u8; 8]>();
        #[cfg(target_endian = "little")]
        let (first, second) = (unsafe { ptr.read_unaligned() }, unsafe {
            ptr.add(1).read_unaligned()
        });
        #[cfg(target_endian = "big")]
        let (first, second) =
            (unsafe { ptr.add(1).read_unaligned() }, unsafe {
                ptr.read_unaligned()
            });
        self.hash = hash_word(
            hash_word(self.hash, u64::from_ne_bytes(first)),
            u64::from_ne_bytes(second),
        );
    }

    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.hash = hash_word(self.hash, i as FixedUsize as u64);
    }

    #[inline]
    fn write_isize(&mut self, i: isize) {
        self.write_i64(i as FixedIsize as i64)
    }
}

/// Hashes the given value with the default value of the specified `Hasher`.
pub fn hash_value<Q, H: Hasher + Default>(value: &Q) -> u64
where
    Q: Hash + ?Sized,
{
    let mut state = H::default();
    value.hash(&mut state);
    state.finish()
}
