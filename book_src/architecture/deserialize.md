# Deserialize

Similarly to `Serialize`, `Deserialize` parameterizes over a deserializer, and converts a type from
its archived form back to its original one. Unlike serialization, deserialization occurs in a single
step and doesn't have an equivalent of a resolver.

> `Deserialize` also parameterizes over the type that is being deserialized into. This allows the
> same archived type to deserialize into multiple different unarchived types depending on what's
> being asked for. This helps enable lots of very powerful abstractions, but might require you to
> use a turbofish or annotate types when deserializing.

This provides a more or less traditional deserialization with the added benefit of being sped up
by having very compiler-friendly representations. It also incurs both the memory and performance
penalties of traditional deserialization, so make sure that it's what you need before you use it.
Deserialization is not required to access archived data as long as you can do so through the
archived versions.

> Even the highest-performance serialization frameworks will hit a deserialization speed limit
> because of the amount of memory allocation that needs to be performed.

A good use for `Deserialize` is deserializing small portions of archives. You can easily traverse
the archived data to locate some subobject, then deserialize just that piece instead of the archive
as a whole. This granular approach provides the benefits of both zero-copy deserialization as well
as traditional deserialization.

## Pooling

Deserializers, like serializers, provide capabilities to objects during deserialization. Most types
don't need to bound their deserializers, but some like `Rc` require special traits in order to
deserialize properly.

The `Pooling` trait controls how pointers which were serialized shared are deserialized. Much like
`Sharing`, `Pooling` holds some mutable state on the deserializer to allow shared pointers to the
same data to coordinate with each other. Using the `Pool` implementation pools these deserialized
shared pointers together, whereas `Unpool` clones them for each instance of the shared pointer.
