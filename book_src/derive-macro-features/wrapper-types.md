# Wrapper types

Wrapper types customize the way that fields of types are archived. In some cases, wrapper types
merely change the default behavior to a preferred alternative. In other cases, wrapper types allow
serializing types which do not have support for rkyv by default.

Annotating a field with `#[rkyv(with = ..)]` will *wrap* that field with the given types when the
struct is serialized or deserialized. There's no performance penalty to wrapping types, but doing
more or less work during serialization and deserialization can affect performance. This excerpt is
from the documentation for `ArchiveWith`:

```rs
#[derive(Archive, Deserialize, Serialize)]
struct Example {
    #[rkyv(with = Incremented)]
    a: i32,
    // Another i32 field, but not incremented this time
    b: i32,
}
```

The `Incremented` wrapper is wrapping `a`, and the definition causes that field to be incremented
in its archived form.
