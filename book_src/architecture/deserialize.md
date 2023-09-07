# Deserialize

Similarly to `Serialize`, [`Deserialize`](https://docs.rs/rkyv/0.7.1/rkyv/trait.Deserialize.html)
parameterizes over and takes a deserializer, and converts a type from its archived form back to its
original one. Unlike serialization, deserialization occurs in a single step and doesn't have an
equivalent of a resolver.

> `Deserialize` also parameterizes over the type that is being deserialized into. This allows the
> same archived type to deserialize into multiple different unarchived types depending on what's
> being asked for. This helps enable lots of very powerful abstractions, but might require you to
> annotate types when deserializing.

This provides a more or less a traditional deserialization with the added benefit of being sped up
somewhat by having very compatible representations. It also incurs both the memory and performance
penalties of traditional deserialization, so make sure that it's what you need before you use it.
Deserialization is not required to access archived data as long as you can do so through the
archived versions.

> Even the highest-performance serialization frameworks will hit a deserialization speed limit
> because of the amount of memory allocation that needs to be performed.

A good use for `Deserialize` is deserializing portions of archives. You can easily traverse the
archived data to locate some subobject, then deserialize just that piece instead of the archive as a
whole. This granular approach provides the benefits of both zero-copy deserialization as well as
traditional deserialization.

## Deserializer

Deserializers, like serializers, provide capabilities to objects during deserialization. Most types
don't bound their deserializers, but some like `Rc` require special deserializers in order to
deserialize memory properly.
