#[cfg(all(not(feature = "big_endian"), target_has_atomic = "16"))]
use rend::{AtomicI16_le, AtomicU16_le};
#[cfg(all(not(feature = "big_endian"), target_has_atomic = "32"))]
use rend::{AtomicI32_le, AtomicU32_le};
#[cfg(all(not(feature = "big_endian"), target_has_atomic = "64"))]
use rend::{AtomicI64_le, AtomicU64_le};

#[cfg(all(feature = "big_endian", target_has_atomic = "16"))]
use rend::{AtomicI16_be, AtomicU16_be};
#[cfg(all(feature = "big_endian", target_has_atomic = "32"))]
use rend::{AtomicI32_be, AtomicU32_be};
#[cfg(all(feature = "big_endian", target_has_atomic = "64"))]
use rend::{AtomicI64_be, AtomicU64_be};

#[cfg(target_has_atomic = "16")]
define_archived_primitive!(ArchivedAtomicI16: AtomicI16, AtomicI16_le, AtomicI16_be);
#[cfg(target_has_atomic = "16")]
define_archived_primitive!(ArchivedAtomicU16: AtomicU16, AtomicU16_le, AtomicU16_be);
#[cfg(target_has_atomic = "32")]
define_archived_primitive!(ArchivedAtomicI32: AtomicI32, AtomicI32_le, AtomicI32_be);
#[cfg(target_has_atomic = "32")]
define_archived_primitive!(ArchivedAtomicU32: AtomicU32, AtomicU32_le, AtomicU32_be);
#[cfg(target_has_atomic = "64")]
define_archived_primitive!(ArchivedAtomicI64: AtomicI64, AtomicI64_le, AtomicI64_be);
#[cfg(target_has_atomic = "64")]
define_archived_primitive!(ArchivedAtomicU64: AtomicU64, AtomicU64_le, AtomicU64_be);

#[cfg(not(feature = "unaligned"))]
macro_rules! define_size_atomics {
    () => {
        /// The native type that `AtomicIsize` is converted to for archiving.
        ///
        /// This will be `AtomicI16`, `AtomicI32`, or `AtomicI64` when the
        /// `pointer_width_16`, `pointer_width_32`, or `pointer_width_64`
        /// features are enabled, respectively. With no pointer width features
        /// enabled, it defaults to `AtomicI32`.
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
        /// features are enabled, respectively. With no pointer width features
        /// enabled, it defaults to `AtomicU32`.
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

#[cfg(all(
    not(feature = "unaligned"),
    any(
        all(target_has_atomic = "16", feature = "pointer_width_16"),
        all(
            target_has_atomic = "32",
            not(any(
                feature = "pointer_width_16",
                feature = "pointer_width_64"
            )),
        ),
        all(target_has_atomic = "64", feature = "pointer_width_64"),
    )
))]
define_size_atomics!();
