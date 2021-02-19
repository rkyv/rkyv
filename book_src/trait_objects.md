# Trait Objects

Trait object serialization is supported through the `rkyv_dyn` crate. This crate is maintained as
part of rkyv, but is separate from the main crate to allow other implementations to be used instead.
This section will focus primarily on the architecture of `rkyv_dyn` and how to use it effectively.

## Core traits

The new traits introduced by `rkyv_dyn` are
[`SerializeDyn`](https://docs.rs/rkyv_dyn/latest/rkyv_dyn/trait.SerializeDyn.html) and
[`DeserializeDyn`](https://docs.rs/rkyv_dyn/latest/rkyv_dyn/trait.DeserializeDyn.html). These are
effectively type-erased versions of `SerializeUnsized` and `DeserializeUnsized` so that the traits
are object-safe. Likewise, it introduces type-erased versions of serializers and deserializers:
[`DynSerializer`](https://docs.rs/rkyv_dyn/latest/rkyv_dyn/trait.DynSerializer.html) and
[`DynDeserializer`](https://docs.rs/rkyv_dyn/latest/rkyv_dyn/trait.DynDeserializer.html). These
attempt to provide the basic functionality required to serialize most types, but may be more or less
capable than custom types require.

## Architecture

It is highly recommended to use the provided
[`archive_dyn`](https://docs.rs/rkyv_dyn/latest/rkyv_dyn/attr.archive_dyn.html) macro to implement
the new traits and set everything up correctly.

Using `archive_dyn` on a trait definition creates another trait definition with supertraits of your
trait and `SerializeDyn`. This "shim" trait is blanket implemented for all types that implement your
trait and `SerializeDyn`, so you should only ever have to implement your trait to use it.

The shim trait should be used everywhere that you have a trait object of your trait that you want to
serialize. By default, it will be named "Serialize" + your trait name. A different approach that
similar libraries take is directly adding `SerializeDyn` as a supertrait of your trait. While more
ergonomic, this approach does not allow the implementation of the trait on types that cannot or
should not implement `SerializeDyn`, so the shim trait approach was favored for `rkyv_dyn`.

When the shim trait is serialized, it stores the type hash of the underlying type in its metadata so
it can get the correct vtable for it when accessed. This requires that all vtables for implementing
types must be known ahead of time, which is when we use `archive_dyn` for the second time.

Using `archive_dyn` on a trait implementation registers the vtable for that implementation with a
global lookup, allowing it to be retrieved later on. Because this process can be slow, the
`vtable_cache` feature allows the vtable lookup to be performed only the first time, then cached
locally for future lookups. This is one of the places where alternate implementations may take a
different approach and choose a different set of benefits and tradeoffs.