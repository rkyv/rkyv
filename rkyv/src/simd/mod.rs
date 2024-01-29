#[path = "generic.rs"]
mod group;

// TODO: add optimized SIMD implementations for sse2 and neon

pub use group::*;

pub const MAX_GROUP_WIDTH: usize = 16;
