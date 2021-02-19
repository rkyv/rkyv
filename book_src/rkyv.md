# rkyv

[rkyv](http://github.com/djkoloski/rkyv) (*archive*) is a zero-copy deserialization framework for
rust.

## Crates

rkyv is composed of three core crates:

- `rkyv`: The base crate, which defines the fundamental traits and provides implementations for core
types.
- `rkyv_dyn`: Builds on the base crate to add support for serializing and deserializing trait
objects.
- `rkyv_typename`: Provides naming for types (used with rkyv_dyn).

The project has three derive crates that are exposed through the core crates:

- `rkyv_derive`
- `rkyv_dyn_derive`
- `rkyv_typename_derive`

There is a test crate: `rkyv_test`, and a benchmarking crate: `rkyv_bench`.

## Links

- [Github repo](https://github.com/djkoloski/rkyv)
- [Docs](https://docs.rs/rkyv)
- [crates.io](https://crates.io/crates/rkyv)

## Sister crates

rkyv has sister crates that are standalone but were designed for use with rkyv:

- [bytecheck](https://github.com/djkoloski/bytecheck): A type validation framework for Rust.
- [ptr_meta](https://github.com/djkoloski/ptr_meta): A radioactive stabilization of the `ptr_meta`
  RFC.