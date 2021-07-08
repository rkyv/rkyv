# Architecture

The core of rkyv is built around
[relative pointers](https://docs.rs/rkyv/latest/rkyv/rel_ptr/struct.RelPtr.html) and three core traits:
[`Archive`](https://docs.rs/rkyv/latest/rkyv/trait.Archive.html),
[`Serialize`](https://docs.rs/rkyv/latest/rkyv/trait.Serialize.html), and
[`Deserialize`](https://docs.rs/rkyv/latest/rkyv/trait.Deserialize.html). Each of these traits has a
corresponding variant that supports unsized types:
[`ArchiveUnsized`](https://docs.rs/rkyv/latest/rkyv/trait.ArchiveUnsized.html),
[`SerializeUnsized`](https://docs.rs/rkyv/latest/rkyv/trait.SerializeUnsized.html), and
[`DeserializeUnsized`](https://docs.rs/rkyv/latest/rkyv/trait.DeserializeUnsized.html).

> A good way to think about it is that sized types are the *foundation* that unsized types are built
> on. That's not a fluke either, rkyv is built precisely so that you can build more complex
> abstractions out of lower-level machinery in a safe and composable way. It's not much different
> from what you normally do while programming!

The system is built to be flexible and can be extended beyond the provided types. For example, the
`rkyv_dyn` crate adds support for trait objects by introducing new traits and defining how they
build up to allow trait objects to be serialized and deserialized.
