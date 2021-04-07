# Serialize

Types implement [`Serialize`](https://docs.rs/rkyv/latest/rkyv/trait.Serialize.html) separately from
`Archive`. `Serialize` creates a resolver for some object, then `Archive` turns the value and that
resolver into an archived type. Having a separate `Serialize` trait is necessary because although a
type may have only one archived representation, different requirements may need to be met in order
to create one.

`Archive` doesn't parameterize over the serializer used to make it, since it shouldn't matter what
serializer an archived type was made with. But `Serialize` does, because it needs to specify the
requirements on its `Serializer` that need to be met for it to create a resolver.

## Serializer

Serializers are types that provide capabilities to objects during serialization. For primitive
types, any serializer can be used because no additional capabilities are required. More complex
types may require the ability to write bytes to the archive, seek throughout the archive, pool
shared resources, and more. rkyv provides serializers with some basic functionality as well as
adapters that add new capabilities to existing serializers.

The most basic serializers used are
[`BufferSerializer`](https://docs.rs/rkyv/latest/rkyv/ser/serializers/struct.BufferSerializer.html)
for serializing into fixed-size byte buffers and
[`WriteSerializer`](https://docs.rs/rkyv/latest/rkyv/ser/serializers/struct.WriteSerializer.html)
for serializing into any [`Writer`](https://doc.rust-lang.org/std/io/trait.Write.html). In many
cases,
[`AlignedSerializer`](https://docs.rs/rkyv/latest/rkyv/ser/serializers/struct.AlignedSerializer.html)
may have better performance.