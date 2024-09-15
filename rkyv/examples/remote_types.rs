use rancor::Failure;
use rkyv::{with::With, Archive, Deserialize, Serialize};

// Assume this is a remote module or crate that you cannot modify.
mod remote {
    // Notably, this type does not implement the rkyv traits
    #[derive(Debug, PartialEq)]
    pub struct Foo {
        pub ch: char,
        pub bytes: [u8; 4],
        pub _uninteresting: u32,
        // ... and even has private fields
        bar: Bar<i32>,
    }

    #[derive(Debug, PartialEq)]
    pub struct Bar<T>(pub T);

    impl Foo {
        // A constructor which is necessary for deserialization because there
        // are private fields.
        pub fn new(
            ch: char,
            bytes: [u8; 4],
            _uninteresting: u32,
            bar: Bar<i32>,
        ) -> Self {
            Self {
                ch,
                bytes,
                _uninteresting,
                bar,
            }
        }

        // The getter for a private field.
        pub fn bar(&self) -> &Bar<i32> {
            &self.bar
        }
    }
}

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

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(remote = remote::Bar<i32>)]
struct BarDef(i32);

impl From<BarDef> for remote::Bar<i32> {
    fn from(BarDef(value): BarDef) -> Self {
        remote::Bar(value)
    }
}

fn main() -> Result<(), Failure> {
    let foo = remote::Foo::new('!', [1, 2, 3, 4], 567, remote::Bar(89));

    // To make use of all the utility functions for serialization, accessing,
    // and deserialization, we can use the `With` type.

    let bytes = rkyv::to_bytes(With::<remote::Foo, FooDef>::cast(&foo))?;
    let archived: &ArchivedFoo = rkyv::access(&bytes)?;
    let deserialized: remote::Foo =
        rkyv::deserialize(With::<ArchivedFoo, FooDef>::cast(archived))?;

    assert_eq!(foo, deserialized);

    // ... or better yet, incorporate the remote type in our own types!

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    struct Baz {
        #[rkyv(with = FooDef)]
        foo: remote::Foo,
    }

    let baz = Baz { foo };

    let bytes = rkyv::to_bytes(&baz)?;
    let archived: &ArchivedBaz = rkyv::access(&bytes)?;
    let deserialized: Baz = rkyv::deserialize(archived)?;

    assert_eq!(baz, deserialized);

    Ok(())
}

#[allow(unused)]
mod another_remote {
    // Another remote type, this time an enum.
    #[non_exhaustive] // <- notice this inconvenience too
    pub enum Qux {
        Unit,
        Tuple(i32),
        Struct { value: bool },
    }
}

#[allow(unused)]
// Enums work similarly
#[derive(Archive, Serialize)]
#[rkyv(remote = another_remote::Qux)]
enum QuxDef {
    // Variants must have the same name and type, e.g. a remote *tuple*
    // variant requires a local *tuple* variant.
    Unit,
    // Same as for actual structs - fields of struct variants may be omitted.
    Struct {},
    // If `Serialize` should be derived and either the remote enum is
    // `#[non_exhaustive]` or any of its variants were omitted (notice the
    // `Tuple(i32)` variant is missing), then the last variant *must* be a
    // unit variant with the `#[rkyv(other)]` attribute.
    #[rkyv(other)]
    Unknown,
}
