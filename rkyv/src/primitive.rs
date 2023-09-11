//! Definitions of archived primitives and type aliases based on enabled
//! features.

#[cfg(feature = "little_endian")]
use crate::rend::{
    char_le, f32_le, f64_le, i128_le, i16_le, i32_le, i64_le, u128_le, u16_le,
    u32_le, u64_le, NonZeroI128_le, NonZeroI16_le, NonZeroI32_le,
    NonZeroI64_le, NonZeroU128_le, NonZeroU16_le, NonZeroU32_le, NonZeroU64_le,
};
#[cfg(all(feature = "little_endian", target_has_atomic = "16"))]
use rend::{AtomicI16_le, AtomicU16_le};
#[cfg(all(feature = "little_endian", target_has_atomic = "32"))]
use rend::{AtomicI32_le, AtomicU32_le};
#[cfg(all(feature = "little_endian", target_has_atomic = "64"))]
use rend::{AtomicI64_le, AtomicU64_le};

#[cfg(feature = "big_endian")]
use crate::rend::{
    char_be, f32_be, f64_be, i128_be, i16_be, i32_be, i64_be, u128_be, u16_be,
    u32_be, u64_be, NonZeroI128_be, NonZeroI16_be, NonZeroI32_be,
    NonZeroI64_be, NonZeroU128_be, NonZeroU16_be, NonZeroU32_be, NonZeroU64_be,
};
#[cfg(all(feature = "big_endian", target_has_atomic = "16"))]
use rend::{AtomicI16_be, AtomicU16_be};
#[cfg(all(feature = "big_endian", target_has_atomic = "32"))]
use rend::{AtomicI32_be, AtomicU32_be};
#[cfg(all(feature = "big_endian", target_has_atomic = "64"))]
use rend::{AtomicI64_be, AtomicU64_be};

macro_rules! define_multibyte_primitive {
    ($archived:ident: $name:ident, $le:ty, $be:ty) => {
        #[doc = concat!(
            "The archived version of `",
            stringify!($)
        )]
        #[cfg(feature = "little_endian")]
        pub type $archived = $le;
        #[cfg(feature = "big_endian")]
        pub type $archived = $be;
    };
}

macro_rules! define_multibyte_primitives {
    ($($archived:ident: $name:ident, $le:ty, $be:ty);* $(;)?) => {
        $(
            define_multibyte_primitive!($archived: $name, $le, $be);
        )*
    }
}

define_multibyte_primitives! {
    ArchivedI16: i16, i16_le, i16_be;
    ArchivedI32: i32, i32_le, i32_be;
    ArchivedI64: i64, i64_le, i64_be;
    ArchivedI128: i128, i128_le, i128_be;
    ArchivedU16: u16, u16_le, u16_be;
    ArchivedU32: u32, u32_le, u32_be;
    ArchivedU64: u64, u64_le, u64_be;
    ArchivedU128: u128, u128_le, u128_be;
    ArchivedF32: f32, f32_le, f32_be;
    ArchivedF64: f64, f64_le, f64_be;
    ArchivedChar: char, char_le, char_be;
}

/// The native type that `isize` is converted to for archiving.
///
/// This will be `i16`, `i32`, or `i64` when the `pointer_width_16`,
/// `pointer_width_32`, or `pointer_width_64` features are enabled,
/// respectively.
pub type FixedIsize = match_pointer_width!(i16, i32, i64);

/// The archived version of `isize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedIsize =
    match_pointer_width!(ArchivedI16, ArchivedI32, ArchivedI64);

/// The native type that `usize` is converted to for archiving.
///
/// This will be `u16`, `u32`, or `u64` when the `pointer_width_16`,
/// `pointer_width_32`, or `pointer_width_64` features are enabled,
/// respectively.
pub type FixedUsize = match_pointer_width!(u16, u32, u64);

/// The archived version of `isize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedUsize =
    match_pointer_width!(ArchivedU16, ArchivedU32, ArchivedU64);

define_multibyte_primitives! {
    ArchivedNonZeroI16: NonZeroI16, NonZeroI16_le, NonZeroI16_be;
    ArchivedNonZeroI32: NonZeroI32, NonZeroI32_le, NonZeroI32_be;
    ArchivedNonZeroI64: NonZeroI64, NonZeroI64_le, NonZeroI64_be;
    ArchivedNonZeroI128: NonZeroI128, NonZeroI128_le, NonZeroI128_be;
    ArchivedNonZeroU16: NonZeroU16, NonZeroU16_le, NonZeroU16_be;
    ArchivedNonZeroU32: NonZeroU32, NonZeroU32_le, NonZeroU32_be;
    ArchivedNonZeroU64: NonZeroU64, NonZeroU64_le, NonZeroU64_be;
    ArchivedNonZeroU128: NonZeroU128, NonZeroU128_le, NonZeroU128_be;
}

/// The native type that `NonZeroIsize` is converted to for archiving.
///
/// This will be `NonZeroI16`, `NonZeroI32`, or `NonZeroI64` when the
/// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64` features are
/// enabled, respectively.
pub type FixedNonZeroIsize = match_pointer_width!(
    ::core::num::NonZeroI16,
    ::core::num::NonZeroI32,
    ::core::num::NonZeroI64,
);

/// The archived version of `NonZeroIsize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedNonZeroIsize = match_pointer_width!(
    ArchviedNonZeroI16,
    ArchivedNonZeroI32,
    ArchivedNonZeroI64
);

/// The native type that `NonZeroUsize` is converted to for archiving.
///
/// This will be `NonZeroU16`, `NonZeroU32`, or `NonZeroU64` when the
/// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64` features are
/// enabled, respectively.
pub type FixedNonZeroUsize = match_pointer_width!(
    ::core::num::NonZeroU16,
    ::core::num::NonZeroU32,
    ::core::num::NonZeroU64,
);

/// The archived version of `NonZeroUsize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedNonZeroUsize = match_pointer_width!(
    ArchivedNonZeroU16,
    ArchivedNonZeroU32,
    ArchivedNonZeroU64
);

#[cfg(target_has_atomic = "16")]
define_multibyte_primitives! {
    ArchivedAtomicI16: AtomicI16, AtomicI16_le, AtomicI16_be;
    ArchivedAtomicU16: AtomicU16, AtomicU16_le, AtomicU16_be;
}
#[cfg(target_has_atomic = "32")]
define_multibyte_primitives! {
    ArchivedAtomicI32: AtomicI32, AtomicI32_le, AtomicI32_be;
    ArchivedAtomicU32: AtomicU32, AtomicU32_le, AtomicU32_be;
}
#[cfg(target_has_atomic = "64")]
define_multibyte_primitives! {
    ArchivedAtomicI64: AtomicI64, AtomicI64_le, AtomicI64_be;
    ArchivedAtomicU64: AtomicU64, AtomicU64_le, AtomicU64_be;
}

macro_rules! define_size_atomics {
    () => {
        /// The native type that `AtomicIsize` is converted to for archiving.
        ///
        /// This will be `AtomicI16`, `AtomicI32`, or `AtomicI64` when the
        /// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64`
        /// features are enabled, respectively.
        pub type FixedAtomicIsize = match_pointer_width!(
            ::core::sync::atomic::AtomicI16,
            ::core::sync::atomic::AtomicI32,
            ::core::sync::atomic::AtomicI64,
        );

        /// The archived version of `AtomicIsize` chosen based on the
        /// currently-enabled `pointer_width_*` feature.
        pub type ArchivedAtomicIsize = match_pointer_width!(
            ArchivedAtomicI16,
            ArchivedAtomicI32,
            ArchivedAtomicI64,
        );

        /// The native type that `AtomicUsize` is converted to for archiving.
        ///
        /// This will be `AtomicU16`, `AtomicU32`, or `AtomicU64` when the
        /// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64`
        /// features are enabled, respectively.
        pub type FixedAtomicUsize = match_pointer_width!(
            ::core::sync::atomic::AtomicU16,
            ::core::sync::atomic::AtomicU32,
            ::core::sync::atomic::AtomicU64,
        );

        /// The archived version of `AtomicUsize` chosen based on the
        /// currently-enabled `pointer_width_*` feature.
        pub type ArchivedAtomicUsize = match_pointer_width!(
            ArchivedAtomicU16,
            ArchivedAtomicU32,
            ArchivedAtomicU64,
        );
    };
}

#[cfg(any(
    all(target_has_atomic = "16", feature = "pointer_width_16"),
    all(target_has_atomic = "32", feature = "pointer_width_32"),
    all(target_has_atomic = "64", feature = "pointer_width_64"),
))]
define_size_atomics!();
