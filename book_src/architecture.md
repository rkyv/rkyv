# Architecture

The core of rkyv is built around
[relative pointers](https://docs.rs/rkyv/latest/rkyv/struct.RelPtr.html) and three core traits:
[`Archive`](https://docs.rs/rkyv/latest/rkyv/trait.Archive.html),
[`Serialize`](https://docs.rs/rkyv/latest/rkyv/trait.Serialize.html), and
[`Deserialize`](https://docs.rs/rkyv/latest/rkyv/trait.Deserialize.html). Each of these traits has a
corresponding variant that supports unsized types:
[`ArchiveRef`](https://docs.rs/rkyv/latest/rkyv/trait.ArchiveRef.html),
[`SerializeRef`](https://docs.rs/rkyv/latest/rkyv/trait.SerializeRef.html), and
[`DeserializeRef`](https://docs.rs/rkyv/latest/rkyv/trait.DeserializeRef.html).

The system is built to be flexible and can be extended beyond the provided types. For example, the
`rkyv_dyn` crate adds support for trait objects by introducing new traits and defining how they
build up to allow types to be serialized and deserialized.
