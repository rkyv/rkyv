# Wrapper types

Wrapper types make it easy to customize the way that fields of types are archived. They make it
easier to adapt rkyv to existing data models, and make serializing and deserializing idiomatic for
even complicated types.

Annotating a field with `#[rkyv(with = ..)]` will *wrap* that field with the given types when the
struct is serialized or deserialized. There's no performance penalty to actually wrap types, but
doing more or less work during serialization and deserialization can affect performance. This
excerpt is from the documentation for [`ArchiveWith`]

[`ArchiveWith`]: https://docs.rs/rkyv/0.7.1/rkyv/with/trait.ArchiveWith.html

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
