# Unarchive

The [`Unarchive`](https://docs.rs/rkyv/latest/rkyv/trait.Unarchive.html) trait provides a method to
convert the archived type back into the original one. This is more or less a traditional
deserialization with the added benefit of being sped up somewhat by having very compatible
representations. The process of unarchiving data incurs both the memory and performance penalties
of traditional serialization, so make sure that it's what you need before you use it.

A good use for `Unarchive` is unarchiving portions of archives. You can easily traverse the archived
data to locate some subobject, then unarchive just that piece instead of the archive as a whole.
This granular approach provides the benefits of both zero-copy deserialization as well as
traditional deserialization.