# Remote derive

Like serde, rkyv also supports _remote derive_. This allows you to easily generate wrapper types to
serialize types from other crates which don't provide rkyv support. Remote derive uses a local
definition of the type to serialize, and generates a wrapper type you can use to serialize that
type.

Remote derive supports getters, wrapper types, and deserialization back to the original type by
providing a `From` impl. This example is from `rkyv/examples/remote_types.rs`:

```rust
// Let's create a local type that will serve as `with`-wrapper for `Foo`.
// Fields must have the same name and type but it's not required to define all
// fields.
#[derive(Archive, Serialize, Deserialize)]
#[rkyv(remote = remote::Foo)] // <-
#[rkyv(archived = ArchivedFoo)]
// ^ not necessary but we might as well replace the default name
// `ArchivedFooDef` with `ArchivedFoo`.
struct FooDef {
    // The field's type implements `Archive` and we don't want to apply any
    // conversion for the archived type so we don't need to specify
    // `#[rkyv(with = ..)]`.
    ch: char,
    // The field is private in the remote type so we need to specify a getter
    // to access it. Also, its type doesn't implement `Archive` so we need
    // to specify a `with`-wrapper too.
    #[rkyv(getter = remote::Foo::bar, with = BarDef)]
    bar: remote::Bar<i32>,
    // The remote `bytes` field is public but we can still customize our local
    // field when using a getter.
    #[rkyv(getter = get_first_byte)]
    first_byte: u8,
}

fn get_first_byte(foo: &remote::Foo) -> u8 {
    foo.bytes[0]
}

// Deriving `Deserialize` with `remote = ..` requires a `From` implementation.
impl From<FooDef> for remote::Foo {
    fn from(value: FooDef) -> Self {
        remote::Foo::new(value.ch, [value.first_byte, 2, 3, 4], 567, value.bar)
    }
}
```
