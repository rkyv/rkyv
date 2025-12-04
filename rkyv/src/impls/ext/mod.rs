// Support for various common crates. These are primarily to get users off the
// ground and build some momentum.

// These are NOT PLANNED to remain in rkyv for the final release. Much like
// serde, these implementations should be moved into their respective crates
// over time. Before adding support for another crate, please consider getting
// rkyv support in the crate instead.

#[cfg(feature = "arrayvec-0_7")]
mod arrayvec_0_7;
#[cfg(feature = "bytes-1")]
mod bytes_1;
#[cfg(any(
    feature = "hashbrown-0_14",
    feature = "hashbrown-0_15",
    feature = "hashbrown-0_16"
))]
mod hashbrown;
#[cfg(feature = "indexmap-2")]
mod indexmap_2;
#[cfg(feature = "smallvec-1")]
mod smallvec_1;
#[cfg(feature = "smol_str-0_2")]
mod smolstr_0_2;
#[cfg(feature = "smol_str-0_3")]
mod smolstr_0_3;
#[cfg(feature = "thin-vec-0_2")]
mod thin_vec_0_2;
#[cfg(feature = "tinyvec-1")]
mod tinyvec_1;
#[cfg(feature = "triomphe-0_1")]
mod triomphe_0_1;
#[cfg(feature = "uuid-1")]
mod uuid_1;
