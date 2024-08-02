#[cfg_attr(
    all(
        target_feature = "sse2",
        any(target_arch = "x86", target_arch = "x86_64"),
        not(miri),
    ),
    path = "sse2.rs"
)]
#[cfg_attr(
    all(
        target_feature = "neon",
        target_arch = "aarch64",
        // NEON intrinsics are currently broken on big-endian targets.
        // See https://github.com/rust-lang/stdarch/issues/1484.
        target_endian = "little",
        not(miri),
    ),
    path = "neon.rs",
)]
#[cfg_attr(
    all(
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
        ))
    ),
    path = "generic.rs",
)]
mod group;

pub use group::*;

pub const MAX_GROUP_WIDTH: usize = 16;
