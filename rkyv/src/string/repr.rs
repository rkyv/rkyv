//! An archived string representation that supports inlining short strings.

use core::{
    marker::PhantomPinned,
    mem,
    ptr::{self, copy_nonoverlapping, write_bytes},
    slice, str,
};

use munge::munge;
use rancor::{Panic, ResultExt as _, Source};

use crate::{
    primitive::{ArchivedIsize, ArchivedUsize, FixedIsize, FixedUsize},
    seal::Seal,
    Place, Portable,
};

#[derive(Clone, Copy, Portable)]
#[rkyv(crate)]
#[repr(C)]
struct OutOfLineRepr {
    len: ArchivedUsize,
    offset: ArchivedIsize,
    _phantom: PhantomPinned,
}

/// The maximum number of bytes that can be inlined.
pub const INLINE_CAPACITY: usize = mem::size_of::<OutOfLineRepr>();
/// The maximum number of bytes that can be out-of-line.
pub const OUT_OF_LINE_CAPACITY: usize = (1 << (FixedUsize::BITS - 2)) - 1;

#[derive(Clone, Copy, Portable)]
#[rkyv(crate)]
#[repr(C)]
struct InlineRepr {
    bytes: [u8; INLINE_CAPACITY],
}

/// An archived string representation that can inline short strings.
#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
pub union ArchivedStringRepr {
    out_of_line: OutOfLineRepr,
    inline: InlineRepr,
}

impl ArchivedStringRepr {
    /// Returns whether the representation is inline.
    #[inline]
    pub fn is_inline(&self) -> bool {
        unsafe { self.inline.bytes[0] & 0xc0 != 0x80 }
    }

    /// Returns the offset of the representation.
    ///
    /// # Safety
    ///
    /// The internal representation must be out-of-line.
    #[inline]
    pub unsafe fn out_of_line_offset(&self) -> isize {
        // SAFETY: The caller has guaranteed that the internal representation is
        // out-of-line
        unsafe { self.out_of_line.offset.to_native() as isize }
    }

    /// Returns a pointer to the bytes of the string.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        if self.is_inline() {
            unsafe { self.inline.bytes.as_ptr() }
        } else {
            unsafe {
                (self as *const Self)
                    .cast::<u8>()
                    .offset(self.out_of_line_offset())
            }
        }
    }

    /// Returns a mutable pointer to the bytes of the string.
    #[inline]
    pub fn as_mut_ptr(this: Seal<'_, Self>) -> *mut u8 {
        let this = unsafe { this.unseal_unchecked() };
        if this.is_inline() {
            unsafe { this.inline.bytes.as_mut_ptr() }
        } else {
            unsafe {
                (this as *mut Self)
                    .cast::<u8>()
                    .offset(this.out_of_line_offset())
            }
        }
    }

    /// Returns the length of the string.
    #[inline]
    pub fn len(&self) -> usize {
        if self.is_inline() {
            unsafe {
                self.inline
                    .bytes
                    .iter()
                    .position(|b| *b == 0xff)
                    .unwrap_or(INLINE_CAPACITY)
            }
        } else {
            let len = unsafe { self.out_of_line.len.to_native() };
            // Little-endian: remove the 7th and 8th bits
            #[cfg(not(feature = "big_endian"))]
            let len = (len & 0b0011_1111) | ((len & !0xff) >> 2);
            // Big-endian: remove the top two bits
            #[cfg(feature = "big_endian")]
            let len = len & (FixedUsize::MAX >> 2);
            len as usize
        }
    }

    /// Returns whether the string is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a pointer to the string as a `str`.
    #[inline]
    pub fn as_str_ptr(&self) -> *const str {
        ptr_meta::from_raw_parts(self.as_ptr().cast(), self.len())
    }

    /// Returns a slice of the bytes of the string.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Returns a mutable slice of the bytes of the string.
    #[inline]
    pub fn as_bytes_seal(this: Seal<'_, Self>) -> Seal<'_, [u8]> {
        let len = this.len();
        let slice =
            unsafe { slice::from_raw_parts_mut(Self::as_mut_ptr(this), len) };
        Seal::new(slice)
    }

    /// Returns a reference to the string as a `str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.as_bytes()) }
    }

    /// Returns a mutable reference to the string as a `str`.
    #[inline]
    pub fn as_str_seal(this: Seal<'_, Self>) -> Seal<'_, str> {
        let bytes =
            unsafe { Seal::unseal_unchecked(Self::as_bytes_seal(this)) };
        Seal::new(unsafe { str::from_utf8_unchecked_mut(bytes) })
    }

    /// Emplaces a new inline representation for the given `str`.
    ///
    /// This function is guaranteed not to write any uninitialized bytes to
    /// `out`.
    ///
    /// # Safety
    ///
    /// - The length of `value` must be less than or equal to
    ///   [`INLINE_CAPACITY`].
    /// - `out` must point to a valid location to write the inline
    ///   representation.
    #[inline]
    pub unsafe fn emplace_inline(value: &str, out: *mut Self) {
        debug_assert!(value.len() <= INLINE_CAPACITY);

        // SAFETY: The caller has guaranteed that `out` points to a
        // dereferenceable location.
        let out_bytes = unsafe { ptr::addr_of_mut!((*out).inline.bytes) };

        // SAFETY: The caller has guaranteed that the length of `value` is less
        // than or equal to `INLINE_CAPACITY`. We know that `out_bytes` is a
        // valid pointer to bytes because it is a subfield of `out` which the
        // caller has guaranteed points to a valid location.
        unsafe {
            write_bytes(out_bytes, 0xff, 1);
            copy_nonoverlapping(
                value.as_bytes().as_ptr(),
                out_bytes.cast(),
                value.len(),
            );
        }
    }

    /// Emplaces a new out-of-line representation for the given `str`.
    ///
    /// # Safety
    ///
    /// The length of `str` must be greater than [`INLINE_CAPACITY`] and less
    /// than or equal to [`OUT_OF_LINE_CAPACITY`].
    pub unsafe fn try_emplace_out_of_line<E: Source>(
        value: &str,
        target: usize,
        out: Place<Self>,
    ) -> Result<(), E> {
        let (len, offset) = unsafe {
            munge! {
                let ArchivedStringRepr {
                    out_of_line: OutOfLineRepr { len, offset, _phantom: _ }
                } = out;
            }
            (len, offset)
        };

        let l = value.len() as FixedUsize;
        // Little-endian: insert 10 as the 7th and 8th bits
        #[cfg(not(feature = "big_endian"))]
        let l = (l & 0b0011_1111) | 0b1000_0000 | ((l & !0b0011_1111) << 2);
        // Big-endian: set the top two bits to 10
        #[cfg(feature = "big_endian")]
        let l = l & (FixedUsize::MAX >> 2) | (1 << FixedUsize::BITS - 1);
        len.write(ArchivedUsize::from_native(l));

        let off = crate::rel_ptr::signed_offset(out.pos(), target)?;
        offset.write(ArchivedIsize::from_native(off as FixedIsize));

        Ok(())
    }

    /// Emplaces a new out-of-line representation for the given `str`.
    ///
    /// # Panics
    ///
    /// - The offset calculated for the repr does not fit in an `isize`
    /// - The offset calculated for the repr exceeds the offset storage
    ///
    /// # Safety
    ///
    /// The length of `str` must be greater than [`INLINE_CAPACITY`] and less
    /// than or equal to [`OUT_OF_LINE_CAPACITY`].
    #[inline]
    pub unsafe fn emplace_out_of_line(
        value: &str,
        target: usize,
        out: Place<Self>,
    ) {
        // SAFETY: The safety conditions for `emplace_out_of_line()` are the
        // same as the safety conditions for `try_emplace_out_of_line()`.
        unsafe {
            Self::try_emplace_out_of_line::<Panic>(value, target, out)
                .always_ok()
        }
    }
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use core::{error::Error, fmt};

    use bytecheck::{rancor::Fallible, CheckBytes};
    use rancor::fail;

    /// An error resulting from an invalid string representation.
    ///
    /// Strings that are inline must have a length of at most
    /// [`INLINE_CAPACITY`].
    #[derive(Debug)]
    pub struct CheckStringReprError;

    impl fmt::Display for CheckStringReprError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "String representation was out-of-line but the length was too \
                 short",
            )
        }
    }

    impl Error for CheckStringReprError {}

    unsafe impl<C> CheckBytes<C> for ArchivedStringRepr
    where
        C: Fallible + ?Sized,
        C::Error: Source,
    {
        unsafe fn check_bytes(
            value: *const Self,
            _: &mut C,
        ) -> Result<(), C::Error> {
            // SAFETY: The fields of `ArchivedStringRepr` are always valid for
            // every bit pattern.
            let repr = unsafe { &*value };

            if !repr.is_inline() && repr.len() <= INLINE_CAPACITY {
                fail!(CheckStringReprError);
            } else {
                Ok(())
            }
        }
    }
};
