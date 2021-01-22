# Extensions

From time to time, the basic capabilities that rkyv provides may not be enough to serialize or
deserialize a new type. Luckily, it's easy to extend the capabilities of rkyv to support new
functionality through trait extensions.

## Fallible

The most basic constraint that serializers and deserializers must satisfy is
[`Fallible`](https://docs.rs/rkyv/latest/rkyv/trait.Fallible.html). `Fallible` types have a single
error type that can be produced by their methods, much like `std::io`. All special functionality
that serializers and deserializers have come as extensions on top of `Fallible`.

## Serializers

rkyv provides three serializer extensions:

- [`Serializer`](https://docs.rs/rkyv/latest/rkyv/ser/trait.Serializer.html), which provides the
  basic functionality to serialize types that contain pointers and nonlocal data.
- [`SharedSerializer`](https://docs.rs/rkyv/latest/rkyv/ser/trait.SharedSerializer.html), which adds
  support for serializing shared pointers like `Rc` and `Arc`.
- [`SeekSerializer`](https://docs.rs/rkyv/latest/rkyv/ser/trait.SeekSerializer.html), which adds
  support for rooted archives and seeking.

## Deserializers

rkyv also provides two deserializer extensions:

- [`Deserializer`](https://docs.rs/rkyv/latest/rkyv/de/trait.Deserializer.html), which provides the
  basic functionality to allocate memory for deserializing types that contain relative pointers.
- [`SharedDeserializer`](https://docs.rs/rkyv/latest/rkyv/de/trait.SharedDeserializer.html), which
  adds support for deserializing shared pointers like `Rc` and `Arc`.

## Validation

Similarly to serialization and deserialization, rkyv also has extensions for `CheckBytes` to support
some of the extended types. These types can be found in the
[`validation`](https://docs.rs/rkyv/latest/rkyv/validation/index.html) module, and most use cases
should be covered by
[`DefaultArchiveValidator`](https://docs.rs/rkyv/latest/rkyv/validation/type.DefaultArchiveValidator.html).
