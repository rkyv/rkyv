//! # rkyv
//!
//! rkyv (*archive*) is a zero-copy deserialization framework for Rust.
//!
//! It's similar to other zero-copy deserialization frameworks such as
//! [Cap'n Proto](https://capnproto.org) and
//! [FlatBuffers](https://google.github.io/flatbuffers). However, while the
//! former have external schemas and heavily restricted data types, rkyv allows
//! all serialized types to be defined in code and can serialize a wide variety
//! of types that the others cannot. Additionally, rkyv is designed to have
//! little to no overhead, and in most cases will perform exactly the same as
//! native types.
//!
//! ## Design
//!
//! Like [serde](https://serde.rs), rkyv uses Rust's powerful trait system to
//! serialize data without the need for reflection. Despite having a wide array
//! of features, you also only pay for what you use. If your data checks out,
//! the serialization process can be as simple as a `memcpy`! Like serde, this
//! allows rkyv to perform at speeds similar to handwritten serializers.
//!
//! Unlike serde, rkyv produces data that is guaranteed deserialization free. If
//! you wrote your data to disk, you can just `mmap` your file into memory, cast
//! a pointer, and your data is ready to use. This makes it ideal for
//! high-performance and IO-bound applications.
//!
//! Limited data mutation is supported through `Pin` APIs, and archived values
//! can be truly deserialized with [`Deserialize`] if full mutation capabilities
//! are needed.
//!
//! [The book](https://rkyv.org) has more details on the design and capabilities
//! of rkyv.
//!
//! ## Type support
//!
//! rkyv has a hashmap implementation that is built for zero-copy
//! deserialization, so you can serialize your hashmaps with abandon. The
//! implementation performs perfect hashing with the compress, hash and displace
//! algorithm to use as little memory as possible while still performing
//! fast lookups.
//!
//! It also comes with a B+ tree implementation that is built for maximum
//! performance by splitting data into easily-pageable 4KB segments. This makes
//! it perfect for building immutable databases and structures for bulk data.
//!
//! rkyv also has support for contextual serialization, deserialization, and
//! validation. It can properly serialize and deserialize shared pointers like
//! `Rc` and `Arc`, and can be extended to support custom contextual types.
//!
//! Finally, rkyv makes it possible to serialize trait objects and use them *as
//! trait objects* without deserialization. See the `rkyv_dyn` crate for more
//! details.
//!
//! ## Tradeoffs
//!
//! While rkyv is a great format for final data, it lacks a full schema system
//! and isn't well equipped for data migration and schema upgrades. If your use
//! case requires these capabilities, you may need additional libraries the
//! build these features on top of rkyv. You can use other serialization
//! frameworks like serde with the same types as rkyv conflict-free.
//!
//! ## Features
//!
//! - `alloc`: Enables types that require the `alloc` crate. Enabled by default.
//! - `little_endian`: Forces archives into a little-endian format. This
//!   guarantees cross-endian compatibility optimized for little-endian
//!   architectures.
//! - `big_endian`: Forces archives into a big-endian format. This guarantees
//!   cross-endian compatibility optimized for big-endian architectures.
//! - `size_16`: Archives integral `*size` types as 16-bit integers. This is
//!   intended to be used only for small archives and may not handle large, more
//!   general data.
//! - `size_32`: Archives integral `*size` types as 32-bit integers. Enabled by
//!   default.
//! - `size_64`: Archives integral `*size` types as 64-bit integers. This is
//!   intended to be used only for very large archives and may cause unnecessary
//!   data bloat.
//! - `std`: Enables standard library support. Enabled by default.
//! - `bytecheck`: Enables validation support through `bytecheck`.
//!
//! ## Crate support
//!
//! Some common crates need to be supported by rkyv before an official
//! integration has been made. Support is provided by rkyv for these crates, but
//! in the future crates should depend on rkyv and provide their own
//! implementations. The crates that already have support provided by rkyv
//! should work toward integrating the implementations into themselves.
//!
//! Crates supported by rkyv:
//!
//! - [`indexmap`](https://docs.rs/indexmap)
//! - [`rend`](https://docs.rs/rend) *Enabled automatically when using
//!   endian-specific archive features.*
//! - [`tinyvec`](https://docs.rs/tinyvec)
//! - [`uuid`](https://docs.rs/uuid)
//!
//! Support for each of these crates can be enabled with a feature of the same
//! name. Additionally, the following external crate features are available:
//!
//! - `uuid_std`: Enables the `std` feature in `uuid`.
//!
//! ## Examples
//!
//! - See [`Archive`] for examples of how to use rkyv through the derive macro
//!   and manual implementation.
//! - For more details on the derive macro and its capabilities, see
//!   [`Archive`](macro@Archive).
//! - Fully worked examples using rkyv are available in the source repository's
//!   [`examples` directory](https://github.com/rkyv/rkyv/tree/master/examples).

// Crate attributes

#![deny(
    rustdoc::broken_intra_doc_links,
    missing_docs,
    rustdoc::missing_crate_level_docs
)]
#![cfg_attr(not(feature = "std"), no_std)]
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

// Re-exports

#[cfg(feature = "bytecheck")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "bytecheck")))]
pub use ::bytecheck;
pub use ::munge;
pub use ::ptr_meta;
pub use ::rancor;
pub use ::rend;
pub use ::rkyv_derive::{Archive, Deserialize, Portable, Serialize};

// Modules

mod alias;
#[macro_use]
mod _macros;
#[cfg(feature = "bitvec")]
pub mod bitvec;
pub mod boxed;
pub mod collections;
pub mod de;
// This is pretty unfortunate. CStr doesn't rely on the rest of std, but it's
// not in core. If CStr ever gets moved into `core` then this module will no
// longer need cfg(feature = "std")
#[cfg(feature = "std")]
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

#[cfg(feature = "alloc")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "alloc")))]
#[doc(inline)]
pub use util::{from_bytes_unchecked, to_bytes};
#[cfg(all(feature = "bytecheck", feature = "alloc"))]
#[cfg_attr(
    doc_cfg,
    doc(cfg(all(feature = "bytecheck", feature = "alloc")))
)]
#[doc(inline)]
pub use validation::util::from_bytes;
#[cfg(feature = "bytecheck")]
#[cfg_attr(doc_cfg, doc(cfg(feature = "bytecheck")))]
#[doc(inline)]
pub use validation::util::{access, access_mut};

#[doc(inline)]
pub use crate::{
    alias::*,
    place::Place,
    traits::*,
    util::{access_unchecked, access_unchecked_mut, deserialize, serialize},
};

// Check endianness feature flag settings

#[cfg(all(feature = "little_endian", feature = "big_endian"))]
core::compiler_error!(
    "\"little_endian\" and \"big_endian\" are mutually-exclusive features. \
     You may need to set `default-features = false` or compile with \
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
