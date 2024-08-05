//! Definitions of archived primitives and type aliases based on enabled
//! features.

// Unaligned big-endian
#[cfg(all(feature = "unaligned", feature = "big_endian"))]
use crate::rend::unaligned::{
    char_ube, f32_ube, f64_ube, i128_ube, i16_ube, i32_ube, i64_ube, u128_ube,
    u16_ube, u32_ube, u64_ube, NonZeroI128_ube, NonZeroI16_ube, NonZeroI32_ube,
    NonZeroI64_ube, NonZeroU128_ube, NonZeroU16_ube, NonZeroU32_ube,
    NonZeroU64_ube,
};
// Unaligned little-endian
#[cfg(all(feature = "unaligned", not(feature = "big_endian")))]
use crate::rend::unaligned::{
    char_ule, f32_ule, f64_ule, i128_ule, i16_ule, i32_ule, i64_ule, u128_ule,
    u16_ule, u32_ule, u64_ule, NonZeroI128_ule, NonZeroI16_ule, NonZeroI32_ule,
    NonZeroI64_ule, NonZeroU128_ule, NonZeroU16_ule, NonZeroU32_ule,
    NonZeroU64_ule,
};
// Aligned big-endian
#[cfg(all(not(feature = "unaligned"), feature = "big_endian"))]
use crate::rend::{
    char_be, f32_be, f64_be, i128_be, i16_be, i32_be, i64_be, u128_be, u16_be,
    u32_be, u64_be, NonZeroI128_be, NonZeroI16_be, NonZeroI32_be,
    NonZeroI64_be, NonZeroU128_be, NonZeroU16_be, NonZeroU32_be, NonZeroU64_be,
};
// Aligned little-endian
#[cfg(all(not(feature = "unaligned"), not(feature = "big_endian")))]
use crate::rend::{
    char_le, f32_le, f64_le, i128_le, i16_le, i32_le, i64_le, u128_le, u16_le,
    u32_le, u64_le, NonZeroI128_le, NonZeroI16_le, NonZeroI32_le,
    NonZeroI64_le, NonZeroU128_le, NonZeroU16_le, NonZeroU32_le, NonZeroU64_le,
};

#[rustfmt::skip]
macro_rules! define_archived_type_alias {
    ($archived:ident: $name:ident, $ty:ty) => {
        #[doc = concat!(
            "The archived version of `",
            stringify!($name),
            "`.",
        )]
        pub type $archived = $ty;
    };
}

macro_rules! define_archived_primitive {
    ($archived:ident: $name:ident, $le:ty, $be:ty) => {
        #[cfg(not(feature = "big_endian"))]
        define_archived_type_alias!($archived: $name, $le);
        #[cfg(feature = "big_endian")]
        define_archived_type_alias!($archived: $name, $be);
    }
}

macro_rules! define_multibyte_primitive {
    ($archived:ident: $name:ident, $le:ty, $ule:ty, $be:ty, $ube:ty) => {
        #[cfg(not(feature = "unaligned"))]
        define_archived_primitive!($archived: $name, $le, $be);
        #[cfg(feature = "unaligned")]
        define_archived_primitive!($archived: $name, $ule, $ube);
    };
}

macro_rules! define_multibyte_primitives {
    (
        $($archived:ident: $name:ident, $le:ty, $ule:ty, $be:ty, $ube:ty);*
        $(;)?
    ) => {
        $(
            define_multibyte_primitive!($archived: $name, $le, $ule, $be, $ube);
        )*
    }
}

define_multibyte_primitives! {
    ArchivedI16: i16, i16_le, i16_ule, i16_be, i16_ube;
    ArchivedI32: i32, i32_le, i32_ule, i32_be, i32_ube;
    ArchivedI64: i64, i64_le, i64_ule, i64_be, i64_ube;
    ArchivedI128: i128, i128_le, i128_ule, i128_be, i128_ube;
    ArchivedU16: u16, u16_le, u16_ule, u16_be, u16_ube;
    ArchivedU32: u32, u32_le, u32_ule, u32_be, u32_ube;
    ArchivedU64: u64, u64_le, u64_ule, u64_be, u64_ube;
    ArchivedU128: u128, u128_le, u128_ule, u128_be, u128_ube;
    ArchivedF32: f32, f32_le, f32_ule, f32_be, f32_ube;
    ArchivedF64: f64, f64_le, f64_ule, f64_be, f64_ube;
    ArchivedChar: char, char_le, char_ule, char_be, char_ube;
}

/// The native type that `isize` is converted to for archiving.
///
/// This will be `i16`, `i32`, or `i64` when the `pointer_width_16`,
/// `pointer_width_32`, or `pointer_width_64` features are enabled,
/// respectively. With no pointer width features enabled, it defaults to `i32`.
pub type FixedIsize = match_pointer_width!(i16, i32, i64);

/// The archived version of `isize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedIsize =
    match_pointer_width!(ArchivedI16, ArchivedI32, ArchivedI64);

/// The native type that `usize` is converted to for archiving.
///
/// This will be `u16`, `u32`, or `u64` when the `pointer_width_16`,
/// `pointer_width_32`, or `pointer_width_64` features are enabled,
/// respectively. With no pointer width features enabled, it defaults to `u32`.
pub type FixedUsize = match_pointer_width!(u16, u32, u64);

/// The archived version of `isize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedUsize =
    match_pointer_width!(ArchivedU16, ArchivedU32, ArchivedU64);

define_multibyte_primitives! {
    ArchivedNonZeroI16:
        NonZeroI16,
        NonZeroI16_le,
        NonZeroI16_ule,
        NonZeroI16_be,
        NonZeroI16_ube;

    ArchivedNonZeroI32:
        NonZeroI32,
        NonZeroI32_le,
        NonZeroI32_ule,
        NonZeroI32_be,
        NonZeroI32_ube;

    ArchivedNonZeroI64:
        NonZeroI64,
        NonZeroI64_le,
        NonZeroI64_ule,
        NonZeroI64_be,
        NonZeroI64_ube;

    ArchivedNonZeroI128:
        NonZeroI128,
        NonZeroI128_le,
        NonZeroI128_ule,
        NonZeroI128_be,
        NonZeroI128_ube;

    ArchivedNonZeroU16:
        NonZeroU16,
        NonZeroU16_le,
        NonZeroU16_ule,
        NonZeroU16_be,
        NonZeroU16_ube;

    ArchivedNonZeroU32:
        NonZeroU32,
        NonZeroU32_le,
        NonZeroU32_ule,
        NonZeroU32_be,
        NonZeroU32_ube;

    ArchivedNonZeroU64:
        NonZeroU64,
        NonZeroU64_le,
        NonZeroU64_ule,
        NonZeroU64_be,
        NonZeroU64_ube;

    ArchivedNonZeroU128:
        NonZeroU128,
        NonZeroU128_le,
        NonZeroU128_ule,
        NonZeroU128_be,
        NonZeroU128_ube;
}

/// The native type that `NonZeroIsize` is converted to for archiving.
///
/// This will be `NonZeroI16`, `NonZeroI32`, or `NonZeroI64` when the
/// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64` features are
/// enabled, respectively. With no pointer width features enabled, it defaults
/// to `NonZeroI32`.
pub type FixedNonZeroIsize = match_pointer_width!(
    ::core::num::NonZeroI16,
    ::core::num::NonZeroI32,
    ::core::num::NonZeroI64,
);

/// The archived version of `NonZeroIsize` chosen based on the currently-enabled
/// `pointer_width_*` feature.
pub type ArchivedNonZeroIsize = match_pointer_width!(
    ArchivedNonZeroI16,
    ArchivedNonZeroI32,
    ArchivedNonZeroI64
);

/// The native type that `NonZeroUsize` is converted to for archiving.
///
/// This will be `NonZeroU16`, `NonZeroU32`, or `NonZeroU64` when the
/// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64` features are
/// enabled, respectively. With no pointer width features enabled, it defaults
/// to `NonZeroU32`.
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
