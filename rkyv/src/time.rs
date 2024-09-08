//! Archived versions of `time` types.

use crate::{
    primitive::{ArchivedU32, ArchivedU64},
    Portable,
};

/// An archived [`Duration`](core::time::Duration).
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Portable,
)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedDuration {
    secs: ArchivedU64,
    nanos: ArchivedU32,
}

const NANOS_PER_SEC: u32 = 1_000_000_000;
const NANOS_PER_MILLI: u32 = 1_000_000;
const NANOS_PER_MICRO: u32 = 1_000;
const MILLIS_PER_SEC: u64 = 1_000;
const MICROS_PER_SEC: u64 = 1_000_000;

impl ArchivedDuration {
    /// Returns the number of _whole_ seconds contained by this
    /// `ArchivedDuration`.
    ///
    /// The returned value does not include the fractional (nanosecond) part of
    /// the duration, which can be obtained using [`subsec_nanos`].
    ///
    /// [`subsec_nanos`]: ArchivedDuration::subsec_nanos
    #[inline]
    pub const fn as_secs(&self) -> u64 {
        self.secs.to_native()
    }

    /// Returns the fractional part of this `ArchivedDuration`, in whole
    /// milliseconds.
    ///
    /// This method does **not** return the length of the duration when
    /// represented by milliseconds. The returned number always represents a
    /// fractional portion of a second (i.e., it is less than one thousand).
    #[inline]
    pub const fn subsec_millis(&self) -> u32 {
        self.nanos.to_native() / NANOS_PER_MILLI
    }

    /// Returns the fractional part of this `ArchivedDuration`, in whole
    /// microseconds.
    ///
    /// This method does **not** return the length of the duration when
    /// represented by microseconds. The returned number always represents a
    /// fractional portion of a second (i.e., it is less than one million).
    #[inline]
    pub const fn subsec_micros(&self) -> u32 {
        self.nanos.to_native() / NANOS_PER_MICRO
    }

    /// Returns the fractional part of this `Duration`, in nanoseconds.
    ///
    /// This method does **not** return the length of the duration when
    /// represented by nanoseconds. The returned number always represents a
    /// fractional portion of a second (i.e., it is less than one billion).
    #[inline]
    pub const fn subsec_nanos(&self) -> u32 {
        self.nanos.to_native()
    }

    /// Returns the total number of whole milliseconds contained by this
    /// `ArchivedDuration`.
    #[inline]
    pub const fn as_millis(&self) -> u128 {
        self.as_secs() as u128 * MILLIS_PER_SEC as u128
            + (self.subsec_nanos() / NANOS_PER_MILLI) as u128
    }

    /// Returns the total number of whole microseconds contained by this
    /// `ArchivedDuration`.
    #[inline]
    pub const fn as_micros(&self) -> u128 {
        self.as_secs() as u128 * MICROS_PER_SEC as u128
            + (self.subsec_nanos() / NANOS_PER_MICRO) as u128
    }

    /// Returns the total number of nanoseconds contained by this
    /// `ArchivedDuration`.
    #[inline]
    pub const fn as_nanos(&self) -> u128 {
        self.as_secs() as u128 * NANOS_PER_SEC as u128
            + self.subsec_nanos() as u128
    }

    /// Returns the number of seconds contained by this `ArchivedDuration` as
    /// `f64`.
    ///
    /// The returned value does include the fractional (nanosecond) part of the
    /// duration.
    #[inline]
    pub fn as_secs_f64(&self) -> f64 {
        (self.as_secs() as f64)
            + (self.subsec_nanos() as f64) / (NANOS_PER_SEC as f64)
    }

    /// Returns the number of seconds contained by this `ArchivedDuration` as
    /// `f32`.
    ///
    /// The returned value does include the fractional (nanosecond) part of the
    /// duration.
    #[inline]
    pub fn as_secs_f32(&self) -> f32 {
        (self.as_secs() as f32)
            + (self.subsec_nanos() as f32) / (NANOS_PER_SEC as f32)
    }

    /// Constructs an archived duration at the given position.
    ///
    /// This function is guaranteed not to write any uninitialized bytes to
    /// `out`.
    ///
    /// # Safety
    ///
    /// `out` must point to memory suitable for holding an `ArchivedDuration`.
    #[inline]
    pub unsafe fn emplace(secs: u64, nanos: u32, out: *mut ArchivedDuration) {
        use core::ptr::addr_of_mut;

        let out_secs = unsafe { addr_of_mut!((*out).secs) };
        unsafe {
            out_secs.write(ArchivedU64::from_native(secs));
        }
        let out_nanos = unsafe { addr_of_mut!((*out).nanos) };
        unsafe {
            out_nanos.write(ArchivedU32::from_native(nanos));
        }
    }
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::{error::Error, fmt};

    use bytecheck::{
        rancor::{Fallible, Source},
        Verify,
    };
    use rancor::fail;

    use super::ArchivedDuration;

    /// An error resulting from an invalid duration.
    ///
    /// Durations must have a `nanos` field that is less than one billion.
    #[derive(Debug)]
    pub struct DurationError {
        nanos: u32,
    }

    impl fmt::Display for DurationError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "`nanos` field of `Duration` is greater than 1 billion: {}",
                self.nanos,
            )
        }
    }

    impl Error for DurationError {}

    unsafe impl<C> Verify<C> for ArchivedDuration
    where
        C: Fallible + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, _: &mut C) -> Result<(), C::Error> {
            let nanos = self.nanos.to_native();
            if nanos >= 1_000_000_000 {
                fail!(DurationError { nanos });
            } else {
                Ok(())
            }
        }
    }
}
