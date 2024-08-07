#[cfg(all(
    target_feature = "sse2",
    any(target_arch = "x86", target_arch = "x86_64"),
    not(miri),
))]
mod sse2;

#[cfg(all(
    target_feature = "sse2",
    any(target_arch = "x86", target_arch = "x86_64"),
    not(miri),
))]
pub use self::sse2::*;

#[cfg(all(
    target_feature = "neon",
    target_arch = "aarch64",
    // NEON intrinsics are currently broken on big-endian targets.
    // See https://github.com/rust-lang/stdarch/issues/1484.
    target_endian = "little",
    not(miri),
))]
mod neon;

#[cfg(all(
    target_feature = "neon",
    target_arch = "aarch64",
    // NEON intrinsics are currently broken on big-endian targets.
    // See https://github.com/rust-lang/stdarch/issues/1484.
    target_endian = "little",
    not(miri),
))]
pub use self::neon::*;

#[cfg(all(
    not(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri),
    )),
    not(all(
        target_feature = "neon",
        target_arch = "aarch64",
        // NEON intrinsics are currently broken on big-endian targets.
        // See https://github.com/rust-lang/stdarch/issues/1484.
        target_endian = "little",
        not(miri),
    )),
))]
mod generic;

#[cfg(all(
    not(all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri),
    )),
    not(all(
        target_feature = "neon",
        target_arch = "aarch64",
        // NEON intrinsics are currently broken on big-endian targets.
        // See https://github.com/rust-lang/stdarch/issues/1484.
        target_endian = "little",
        not(miri),
    )),
))]
pub use self::generic::*;

pub const MAX_GROUP_WIDTH: usize = 16;
