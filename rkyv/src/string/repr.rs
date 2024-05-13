//! An archived string representation that supports inlining short strings.

use core::{marker::PhantomPinned, mem, ptr, slice, str};

use munge::munge;
use rancor::{Panic, ResultExt as _, Source};

use crate::{
    primitive::{ArchivedUsize, FixedIsize},
    Place, Portable,
};

const OFFSET_BYTES: usize = mem::size_of::<FixedIsize>();

#[derive(Clone, Copy, Portable)]
#[archive(crate)]
#[repr(C)]
struct OutOfLineRepr {
    len: ArchivedUsize,
    // Offset is always stored in little-endian format to put the sign bit at
    // the end. This representation is optimized for little-endian
    // architectures.
    offset: [u8; OFFSET_BYTES],
    _phantom: PhantomPinned,
}

/// The maximum number of bytes that can be inlined.
pub const INLINE_CAPACITY: usize = mem::size_of::<OutOfLineRepr>() - 1;

#[derive(Clone, Copy, Portable)]
#[archive(crate)]
#[repr(C)]
struct InlineRepr {
    bytes: [u8; INLINE_CAPACITY],
    len: u8,
}

/// An archived string representation that can inline short strings.
#[derive(Portable)]
#[archive(crate)]
#[repr(C)]
pub union ArchivedStringRepr {
    out_of_line: OutOfLineRepr,
    inline: InlineRepr,
}

impl ArchivedStringRepr {
    /// Returns whether the representation is inline.
    #[inline]
    pub fn is_inline(&self) -> bool {
        unsafe { self.inline.len & 0x80 == 0 }
    }

    /// Returns the offset of the representation.
    ///
    /// # Safety
    ///
    /// The internal representation must be out-of-line.
    #[inline]
    pub unsafe fn out_of_line_offset(&self) -> isize {
        // SAFETY: It is always sound to reinterpret the bytes of
        // `ArchivedStringRepr` as `out_of_line` because the two fields of
        // `ArchviedStringRepr` are the same size and every bit pattern of
        // `out_of_line` is valid for it.
        unsafe { FixedIsize::from_le_bytes(self.out_of_line.offset) as isize }
    }

    /// Returns a pointer to the bytes of the string.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        unsafe {
            if self.is_inline() {
                self.inline.bytes.as_ptr()
            } else {
                (self as *const Self)
                    .cast::<u8>()
                    .offset(self.out_of_line_offset())
            }
        }
    }

    /// Returns a mutable pointer to the bytes of the string.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        unsafe {
            if self.is_inline() {
                self.inline.bytes.as_mut_ptr()
            } else {
                (self as *mut Self)
                    .cast::<u8>()
                    .offset(self.out_of_line_offset())
            }
        }
    }

    /// Returns the length of the string.
    #[inline]
    pub fn len(&self) -> usize {
        unsafe {
            if self.is_inline() {
                self.inline.len as usize
            } else {
                self.out_of_line.len.to_native() as usize
            }
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
    pub fn bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.as_ptr(), self.len()) }
    }

    /// Returns a mutable slice of the bytes of the string.
    #[inline]
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.as_mut_ptr(), self.len()) }
    }

    /// Returns a reference to the string as a `str`.
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.bytes()) }
    }

    /// Returns a mutable reference to the string as a `str`.
    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.bytes_mut()) }
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
        // SAFETY: The caller has guaranteed that `out` points to a
        // dereferenceable location.
        let out_bytes = unsafe { ptr::addr_of_mut!((*out).inline.bytes) };
        // SAFETY: The caller has guaranteed that the length of `value` is less
        // than or equal to `INLINE_CAPACITY`. We know that `out_bytes` is a
        // valid pointer to bytes because it is a subfield of `out` which the
        // caller has guaranteed points to a valid location.
        unsafe {
            ptr::copy_nonoverlapping(
                value.as_bytes().as_ptr(),
                out_bytes.cast(),
                value.len(),
            );
        }

        // SAFETY: The caller has guaranteed that `out` points to a
        // dereferenceable location.
        let out_len = unsafe { ptr::addr_of_mut!((*out).inline.len) };
        // SAFETY: `out_len` is properly aligned and valid for writes because it
        // is a pointer to a subfield of `out`, which is also properly aligned
        // and valid for writes.
        unsafe {
            out_len.write(value.len() as u8);
        }
    }

    /// Emplaces a new out-of-line representation for the given `str`.
    ///
    /// # Safety
    ///
    /// The length of `str` must be greater than [`INLINE_CAPACITY`].
    pub unsafe fn try_emplace_out_of_line<E: Source>(
        value: &str,
        target: usize,
        out: Place<Self>,
    ) -> Result<(), E> {
        munge! {
            let ArchivedStringRepr {
                out_of_line: OutOfLineRepr { len, offset, _phantom: _ }
            } = out;
        }
        len.write(ArchivedUsize::from_native(
            value.len().try_into().into_error()?,
        ));

        let off = crate::rel_ptr::signed_offset(out.pos(), target)?;
        offset.write((off as FixedIsize).to_le_bytes());

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
    /// The length of `str` must be greater than [`INLINE_CAPACITY`].
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
    use core::fmt;

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
                "String representation was inline but the length was too large",
            )
        }
    }

    #[cfg(feature = "std")]
    impl std::error::Error for CheckStringReprError {}

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

            if repr.is_inline() && repr.len() > INLINE_CAPACITY {
                fail!(CheckStringReprError);
            } else {
                Ok(())
            }
        }
    }
};
