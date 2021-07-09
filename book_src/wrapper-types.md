# Wrapper types

Wrapper types make it easy to customize the way that fields of types are archived. They make it
easier to adapt rkyv to existing data models, and make serializing and deserializing idiomatic for
even complicated types.

Annotating a field with `#[with(...)]` will *wrap* that field with the given types when the struct
is serialized or deserialized. There's no performance penalty to actually wrap types, but doing more
or less work during serialization and deserialization can affect performance. This excerpt is from
the documentation for [`ArchiveWith`](https://docs.rs/rkyv/0.7.1/rkyv/with/trait.ArchiveWith.html):

```rs
#[derive(Archive, Deserialize, Serialize)]
struct Example {
    #[with(Incremented)]
    a: i32,
    // Another i32 field, but not incremented this time
    b: i32,
}
```

The `Incremented` wrapper is wrapping `a`, and the definition causes that field to be incremented
in its archived form. 

## `With`

The core type behind wrappers is [`With`](https://docs.rs/rkyv/0.7.1/rkyv/with/struct.With.html).
This struct is *transparent*, meaning that it's like another name for the type inside of it. rkyv
uses `With` to wrap your fields when serializing and deserializing, and when you write your own
wrappers they will be used with `With` as well.

See [`ArchiveWith`](https://docs.rs/rkyv/0.7.1/rkyv/with/trait.ArchiveWith.html) for an example of how
to write your own wrapper types.
