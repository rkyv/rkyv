//! rkyv is a zero-copy deserialization framework for Rust.
//!
//! ## Overview
//!
//! rkyv uses Rust's powerful trait system to serialize data without reflection.
//! Many zero-copy deserialization frameworks use external schemas and heavily
//! restrict the available data types. By contrast, rkyv allows all serialized
//! types to be defined in code and can serialize a wide variety of types that
//! other frameworks cannot.
//!
//! rkyv scales to highly-capable as well as highly-restricted environments. Not
//! only does rkyv support "no-std" builds for targets without a standard
//! library implementation, it also supports "no-alloc" builds for targets where
//! allocations cannot be made.
//!
//! rkyv supports limited in-place data mutation, and so can access and update
//! data without ever deserializing back to native types. When rkyv's in-place
//! mutation is too limited, rkyv also provides ergonomic and performant
//! deserialization back into native types.
//!
//! rkyv prioritizes performance, and is one of the fastest serialization
//! frameworks available. All of rkyv's features can be individually enabled and
//! disabled, so you only pay for what you use. Additionally, all of rkyv's
//! zero-copy types are designed to have little to no overhead. In most cases,
//! rkyv's types will have exactly the same performance as native types.
//!
//! See the [rkyv book] for guide-level documentation and usage examples.
//!
//! [rkyv book]: https://rkyv.org
//!
//! ## Components
//!
//! rkyv has [a hash map implementation] that is built for zero-copy
//! deserialization, with the same lookup and iteration performance as the
//! standard library hash maps. The hash map implementation is based on
//! [Swiss Tables] and uses a target-independent version of FxHash to ensure
//! that all targets compute the same hashes.
//!
//! It also has [a B-tree implementation] that has the same performance
//! characteristics as the standard library B-tree maps. Its compact
//! representation and localized data storage is best-suited for very large
//! amounts of data.
//!
//! rkyv supports [shared pointers] by default, and is able to serialize and
//! deserialize them without duplicating the underlying data. Shared pointers
//! which point to the same data when serialized will still point to the same
//! data when deserialized. By default, rkyv only supports non-cyclic data
//! structures.
//!
//! Alongside its [unchecked API], rkyv also provides optional [validation] so
//! you can ensure safety and data integrity at the cost of some overhead.
//! Because checking serialized data can generally be done without allocations,
//! the cost of checking and zero-copy access can be much lower than that of
//! traditional deserialization.
//!
//! rkyv is trait-oriented from top to bottom, and is made to be extended with
//! custom and specialized types. Serialization, deserialization, and
//! validation traits all accept generic context types, making it easy to add
//! new capabilities without degrading ergonomics.
//!
//! [a hash map implementation]: collections::swiss_table::ArchivedHashMap
//! [Swiss Tables]: https://abseil.io/about/design/swisstables
//! [a B-tree implementation]: collections::btree_map::ArchivedBTreeMap
//! [shared pointers]: rc
//! [unchecked API]: access_unchecked
//! [validation]: access
//!
//! ## Features
//!
//! rkyv has several feature flags which can be used to modify its behavior. By
//! default, rkyv enables the `std`, `alloc`, and `bytecheck` features.
//!
//! ### Format control
//!
//! These features control how rkyv formats its serialized data. Enabling and
//! disabling these features may change rkyv's serialized format, and as such
//! can cause previously-serialized data to become unreadable. Enabling format
//! control features that are not the default should be considered a breaking
//! change to rkyv's serialized format.
//!
//! Binaries should consider explicitly choosing format control options from the
//! start, even though doing so is not required. This ensures that developers
//! stay informed about the specific choices being made, and prevents any
//! unexpected compatibility issues with libraries they depend on.
//!
//! Libraries should avoid enabling format control features unless they intend
//! to only support rkyv when those specific format control features are
//! enabled. In general, libraries should be able to support all format control
//! options if they use rkyv's exported types and aliases.
//!
//! #### Endianness
//!
//! If an endianness feature is not enabled, rkyv will use little-endian byte
//! ordering by default.
//!
//! - `little_endian`: Forces data serialization to use little-endian byte
//!   ordering. This optimizes serialized data for little-endian architectures.
//! - `big_endian`: Forces data serialization to use big-endian byte ordering.
//!   This optimizes serialized data for big-endian architectures.
//!
//! #### Alignment
//!
//! If an alignment feature is not enabled, rkyv will use aligned primitives by
//! default.
//!
//! - `aligned`: Forces data serialization to use aligned primitives. This adds
//!   alignment requirements for accessing data and prevents rkyv from working
//!   with unaligned data.
//! - `unaligned`: Forces data serialization to use unaligned primitives. This
//!   removes alignment requirements for accessing data and allows rkyv to work
//!   with unaligned data more easily.
//!
//! #### Pointer width
//!
//! If a pointer width feature is not enabled, rkyv will serialize `isize` and
//! `usize` as 32-bit integers by default.
//!
//! - `pointer_width_16`: Serializes `isize` and `usize` as 16-bit integers.
//!   This is intended to be used only for small data sizes and may not handle
//!   large amounts of data.
//! - `pointer_width_32`: Serializes `isize` and `usize` as 32-bit integers.
//!   This is a good choice for most data, and balances the storage overhead
//!   with support for large data sizes.
//! - `pointer_width_64`: Serializes `isize` and `usize` as 64-bit integers.
//!   This is intended to be used only for extremely large data sizes and may
//!   cause unnecessary data bloat for smaller amounts of data.
//!
//! ### Functionality
//!
//! These features enable more built-in functionality and provide more powerful
//! and ergonomic APIs. Enabling and disabling these features does not change
//! rkyv's serialized format.
//!
//! - `alloc`: Enables support for the `alloc` crate. Enabled by default.
//! - `std`: Enables standard library support. Enabled by default.
//! - `bytecheck`: Enables data validation through `bytecheck`. Enabled by
//!   default.
//!
//! ### Crates
//!
//! rkyv provides integrations for some common crates by default. In the future,
//! crates should depend on rkyv and provide their own integration. Enabling and
//! disabling these features does not change rkyv's serialized format.
//!
//! - [`arrayvec-0_7`](https://docs.rs/arrayvec/0.7)
//! - [`bytes-1`](https://docs.rs/bytes/1)
//! - [`hashbrown-0_14`](https://docs.rs/hashbrown/0.14)
//! - [`hashbrown-0_15`](https://docs.rs/hashbrown/0.15)
//! - [`indexmap-2`](https://docs.rs/indexmap/2)
//! - [`smallvec-1`](https://docs.rs/smallvec/1)
//! - [`smol_str-0_2`](https://docs.rs/smol_str/0.2)
//! - [`smol_str-0_3`](https://docs.rs/smol_str/0.3)
//! - [`thin-vec-0_2`](https://docs.rs/thin-vec/0.2)
//! - [`tinyvec-1`](https://docs.rs/tinyvec/1)
//! - [`triomphe-0_1`](https://docs.rs/triomphe/0.1)
//! - [`uuid-1`](https://docs.rs/uuid/1)
//!
//! ## Compatibility
//!
//! Serialized data can be accessed later as long as:
//!
//! - The underlying schema has not changed
//! - The serialized format has not been changed by format control features
//! - The data was serialized by a semver-compatible version of rkyv

// Crate attributes

#![deny(
    future_incompatible,
    missing_docs,
    nonstandard_style,
    unsafe_op_in_unsafe_fn,
    unused,
    warnings,
    clippy::all,
    clippy::missing_safety_doc,
    // TODO(#114): re-enable this lint after justifying unsafe blocks
    // clippy::undocumented_unsafe_blocks,
    rustdoc::broken_intra_doc_links,
    rustdoc::missing_crate_level_docs
)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(all(docsrs, not(doctest)), feature(doc_cfg))]
#![doc(html_favicon_url = r#"
    data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0
    26.458 26.458'%3E%3Cpath d='M0 0v26.458h26.458V0zm9.175 3.772l8.107 8.106
    2.702-2.702 2.702 13.512-13.512-2.702 2.703-2.702-8.107-8.107z'/%3E
    %3C/svg%3E
"#)]
#![doc(html_logo_url = r#"
    data:image/svg+xml,%3Csvg xmlns="http://www.w3.org/2000/svg" width="100"
    height="100" viewBox="0 0 26.458 26.458"%3E%3Cpath d="M0
    0v26.458h26.458V0zm9.175 3.772l8.107 8.106 2.702-2.702 2.702
    13.512-13.512-2.702 2.703-2.702-8.107-8.107z"/%3E%3C/svg%3E
"#)]
#![cfg_attr(miri, feature(alloc_layout_extra))]

// Extern crates

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;
#[cfg(feature = "std")]
use std as alloc;

// Re-exports
#[cfg(feature = "bytecheck")]
pub use ::bytecheck;
pub use ::munge;
pub use ::ptr_meta;
pub use ::rancor;
pub use ::rend;

// Modules

mod alias;
#[macro_use]
mod _macros;
pub mod api;
pub mod boxed;
pub mod collections;
pub mod de;
pub mod ffi;
pub mod hash;
mod impls;
pub mod net;
pub mod niche;
pub mod ops;
pub mod option;
pub mod place;
mod polyfill;
pub mod primitive;
pub mod rc;
pub mod rel_ptr;
pub mod result;
pub mod seal;
pub mod ser;
mod simd;
pub mod string;
pub mod time;
pub mod traits;
pub mod tuple;
pub mod util;
#[cfg(feature = "bytecheck")]
pub mod validation;
pub mod vec;
pub mod with;

// Exports

#[cfg(all(feature = "bytecheck", feature = "alloc"))]
#[doc(inline)]
pub use api::high::{access, access_mut, from_bytes};
#[cfg(feature = "alloc")]
#[doc(inline)]
pub use api::high::{deserialize, from_bytes_unchecked, to_bytes};

#[doc(inline)]
pub use crate::{
    alias::*,
    api::{access_unchecked, access_unchecked_mut},
    place::Place,
    traits::{
        Archive, ArchiveUnsized, Deserialize, DeserializeUnsized, Portable,
        Serialize, SerializeUnsized,
    },
};

// Check endianness feature flag settings

#[cfg(all(feature = "little_endian", feature = "big_endian"))]
core::compiler_error!(
    "\"little_endian\" and \"big_endian\" are mutually-exclusive features. \
     You may need to set `default-features = false` or compile with \
     `--no-default-features`."
);

// Check alignment feature flag settings

#[cfg(all(feature = "aligned", feature = "unaligned"))]
core::compiler_error!(
    "\"aligned\" and \"unaligned\" are mutually-exclusive features. You may \
     need to set `default-features = false` or compile with \
     `--no-default-features`."
);

// Check pointer width feature flag settings

#[cfg(all(
    feature = "pointer_width_16",
    feature = "pointer_width_32",
    not(feature = "pointer_width_64")
))]
core::compile_error!(
    "\"pointer_width_16\" and \"pointer_width_32\" are mutually-exclusive \
     features. You may need to set `default-features = false` or compile with \
     `--no-default-features`."
);
#[cfg(all(
    feature = "pointer_width_16",
    feature = "pointer_width_64",
    not(feature = "pointer_width_32")
))]
core::compile_error!(
    "\"pointer_width_16\" and \"pointer_width_64\" are mutually-exclusive \
     features. You may need to set `default-features = false` or compile with \
     `--no-default-features`."
);
#[cfg(all(
    feature = "pointer_width_32",
    feature = "pointer_width_64",
    not(feature = "pointer_width_16")
))]
core::compile_error!(
    "\"pointer_width_32\" and \"pointer_width_64\" are mutually-exclusive \
     features. You may need to set `default-features = false` or compile with \
     `--no-default-features`."
);
#[cfg(all(
    feature = "pointer_width_16",
    feature = "pointer_width_32",
    feature = "pointer_width_64"
))]
core::compile_error!(
    "\"pointer_width_16\", \"pointer_width_32\", and \"pointer_width_64\" are \
     mutually-exclusive features. You may need to set `default-features = \
     false` or compile with `--no-default-features`."
);
