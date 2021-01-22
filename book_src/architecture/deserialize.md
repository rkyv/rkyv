# Deserialize

Similarly to `Serialize`, [`Deserialize`](https://docs.rs/rkyv/latest/rkyv/trait.Deserialize.html)
parameterizes over and takes a deserializer, and converts a type from its archived form back to its
original one. Unlike serialization, deserialization occurs in a single step and doesn't have an
equivalent of a resolver.

This provides a more or less a traditional deserialization with the added benefit of being sped up
somewhat by having very compatible representations. It also incurs both the memory and performance
penalties of traditional deserialization, so make sure that it's what you need before you use it.
Deserialization is not required to access archived data as long as you can do so through the
archived versions.

A good use for `Deserialize` is deserializing portions of archives. You can easily traverse the
archived data to locate some subobject, then deserialize just that piece instead of the archive as a
whole. This granular approach provides the benefits of both zero-copy deserialization as well as
traditional deserialization.

## Deserializer

Deserializers, like serializers, provide capabilities to objects during deserialization. The most
basic capability provides the ability to allocate memory, which is required for deserializing
unsized types.
