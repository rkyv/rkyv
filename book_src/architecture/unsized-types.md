# Unsized types

Unlike traditional sized types, unsized types are archived through the
[`ArchiveRef`](https://docs.rs/rkyv/latest/rkyv/trait.ArchiveRef.html) trait. Unlike `Archive`,
which directly stores a type in an archive, `ArchiveRef` stores a reference to a type. Crucially,
that reference can store additional data about the archived type, such as size or length. This
allows unsized types to be archived, such as slices and structs with a trailing slice.

`ArchiveRef` is blanket implemented for all `Archive` types, with a reference type that's just a
relative pointer. This allows `Box` to archive sized and unsized types the same way without extra
annotations or wrapper types.

## Trait objects

The [`rkyv_dyn`](https://docs.rs/rkyv_dyn) crate adds support for trait object serialization and
deserialization. Trait objects can be archived and treated as a trait object while archived. The
crate documentation has examples of how to use it.