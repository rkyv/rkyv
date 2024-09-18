use std::{fmt::Debug, marker::PhantomData, mem::MaybeUninit};

use rancor::{Fallible, Panic, ResultExt, Source, Strategy};
use rkyv::{
    api::low::LowSerializer,
    ser::{allocator::SubAllocator, writer::Buffer, Writer},
    with::{ArchiveWith, DeserializeWith, Map, SerializeWith},
    Archive, Archived, Deserialize, Place, Resolver, Serialize,
};

type ArchivedWith<F, T> = <F as ArchiveWith<T>>::Archived;

fn roundtrip<F, T>(remote: &T)
where
    F: ArchiveWith<T, Archived: CheckedArchived>
        + for<'a, 'b> SerializeWith<T, Serializer<'a, 'b>>
        + DeserializeWith<ArchivedWith<F, T>, T, Strategy<(), Panic>>,
    T: Debug + PartialEq,
{
    let mut bytes = [0_u8; 128];
    let buf = serialize::<F, T>(remote, &mut bytes);
    let archived = access::<F, T>(&buf);
    let deserialized: T =
        F::deserialize_with(archived, Strategy::wrap(&mut ())).always_ok();

    assert_eq!(remote, &deserialized);
}

#[test]
fn named_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote<'a, A> {
        a: u8,
        b: PhantomData<&'a A>,
        c: Option<Foo>,
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote<'a, A>)]
    struct Example<'a, A> {
        a: u8,
        #[rkyv(with = Identity)]
        b: PhantomData<&'a A>,
        #[rkyv(with = Map<FooWrap>)]
        c: Option<Foo>,
    }

    impl<'a, A> From<Example<'a, A>> for Remote<'a, A> {
        fn from(value: Example<'a, A>) -> Self {
            Remote {
                a: value.a,
                b: value.b,
                c: value.c,
            }
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote<'a, A>)]
    struct Partial<'a, A> {
        b: PhantomData<&'a A>,
        #[rkyv(with = Map<FooWrap>)]
        c: Option<Foo>,
    }

    impl<'a, A> From<Partial<'a, A>> for Remote<'a, A> {
        fn from(archived: Partial<'a, A>) -> Self {
            Self {
                a: 42,
                b: archived.b,
                c: archived.c,
            }
        }
    }

    let remote = Remote {
        a: 42,
        b: PhantomData,
        c: Some(Foo::default()),
    };

    roundtrip::<Example<i32>, _>(&remote);
    roundtrip::<Partial<i32>, _>(&remote);
}

#[test]
fn unnamed_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote<'a, A>(u8, PhantomData<&'a A>, Option<Foo>);

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<'a, A>)]
    struct Example<'a, A>(
        u8,
        #[rkyv(with = Identity)] PhantomData<&'a A>,
        #[rkyv(with = Map<FooWrap>)] Option<Foo>,
    );

    impl<'a, A> From<Example<'a, A>> for Remote<'a, A> {
        fn from(value: Example<'a, A>) -> Self {
            Remote(value.0, value.1, value.2)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<'a, A>)]
    struct Partial<'a, A>(
        u8,
        #[rkyv(with = Identity)] PhantomData<&'a A>,
        // Only trailing fields may be omitted for unnamed structs
    );

    impl<'a, A> From<Partial<'a, A>> for Remote<'a, A> {
        fn from(archived: Partial<'a, A>) -> Self {
            Remote(archived.0, archived.1, Some(Foo::default()))
        }
    }

    let remote = Remote(42, PhantomData, Some(Foo::default()));

    roundtrip::<Example<i32>, _>(&remote);
    roundtrip::<Partial<i32>, _>(&remote);
}

#[test]
fn unit_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote;

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote)]
    struct Example;

    impl From<Example> for Remote {
        fn from(_: Example) -> Self {
            Self
        }
    }

    let remote = Remote;
    roundtrip::<Example, _>(&remote);
}

#[test]
fn full_enum() {
    #[derive(Debug, PartialEq)]
    enum Remote<'a, A> {
        A,
        B(u8),
        C {
            a: PhantomData<&'a A>,
            b: Option<Foo>,
        },
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<'a, A>)]
    enum Example<'a, A> {
        A,
        B(u8),
        C {
            #[rkyv(with = Identity)]
            a: PhantomData<&'a A>,
            #[rkyv(with = Map<FooWrap>)]
            b: Option<Foo>,
        },
    }

    impl<'a, A> From<Example<'a, A>> for Remote<'a, A> {
        fn from(value: Example<'a, A>) -> Self {
            match value {
                Example::A => Remote::A,
                Example::B(value) => Remote::B(value),
                Example::C { a, b } => Remote::C { a, b },
            }
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<'a, A>)]
    // If a variant is missing (or the remote type is `#[non_exhaustive]`), one
    // *unit* variant must be denoted with `#[rkyv(other)]`.
    enum Partial<'a, A> {
        A,
        C {
            a: PhantomData<&'a A>,
        },
        #[rkyv(other)]
        Other,
    }

    impl<'a, A> From<Partial<'a, A>> for Remote<'a, A> {
        fn from(archived: Partial<'a, A>) -> Self {
            match archived {
                Partial::A => Remote::A,
                Partial::C { a } => Remote::C {
                    a,
                    b: Some(Foo::default()),
                },
                Partial::Other => Remote::B(42),
            }
        }
    }

    for remote in [
        Remote::A,
        Remote::B(42),
        Remote::C {
            a: PhantomData,
            b: Some(Foo::default()),
        },
    ] {
        roundtrip::<Example<i32>, _>(&remote);
        roundtrip::<Partial<i32>, _>(&remote);
    }
}

#[test]
fn named_struct_private() {
    mod remote {
        #[derive(Copy, Clone, Debug, Default, PartialEq)]
        pub struct Remote {
            inner: [u8; 4],
        }

        impl Remote {
            pub fn new(inner: [u8; 4]) -> Self {
                Self { inner }
            }

            pub fn inner(&self) -> [u8; 4] {
                self.inner
            }

            pub fn inner_ref(&self) -> &[u8; 4] {
                &self.inner
            }
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    struct ExampleByRef {
        #[rkyv(getter = remote::Remote::inner)]
        inner: [u8; 4],
    }

    impl From<ExampleByRef> for remote::Remote {
        fn from(value: ExampleByRef) -> Self {
            remote::Remote::new(value.inner)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    struct ExampleThroughRef {
        #[rkyv(getter = remote::Remote::inner_ref)]
        inner: [u8; 4],
    }

    impl From<ExampleThroughRef> for remote::Remote {
        fn from(value: ExampleThroughRef) -> Self {
            remote::Remote::new(value.inner)
        }
    }

    let remote = remote::Remote::default();
    roundtrip::<ExampleByRef, _>(&remote);
    roundtrip::<ExampleThroughRef, _>(&remote);
}

#[test]
fn unnamed_struct_private() {
    mod remote {
        #[derive(Copy, Clone, Debug, Default, PartialEq)]
        pub struct Remote([u8; 4]);

        impl Remote {
            pub fn new(inner: [u8; 4]) -> Self {
                Self(inner)
            }

            pub fn inner(&self) -> [u8; 4] {
                self.0
            }

            pub fn inner_ref(&self) -> &[u8; 4] {
                &self.0
            }
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    struct ExampleByRef(#[rkyv(getter = remote::Remote::inner)] [u8; 4]);

    impl From<ExampleByRef> for remote::Remote {
        fn from(value: ExampleByRef) -> Self {
            remote::Remote::new(value.0)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    struct ExampleThroughRef(
        #[rkyv(getter = remote::Remote::inner_ref)] [u8; 4],
    );

    impl From<ExampleThroughRef> for remote::Remote {
        fn from(value: ExampleThroughRef) -> Self {
            remote::Remote::new(value.0)
        }
    }

    let remote = remote::Remote::default();
    roundtrip::<ExampleByRef, _>(&remote);
    roundtrip::<ExampleThroughRef, _>(&remote);
}

#[cfg(feature = "bytecheck")]
pub trait CheckedArchived:
    for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::low::LowValidator<'a, Panic>>
{
}

#[cfg(feature = "bytecheck")]
impl<
        Archived: for<'a> rkyv::bytecheck::CheckBytes<
            rkyv::api::low::LowValidator<'a, Panic>,
        >,
    > CheckedArchived for Archived
{
}

#[cfg(not(feature = "bytecheck"))]
pub trait CheckedArchived {}

#[cfg(not(feature = "bytecheck"))]
impl<Archived> CheckedArchived for Archived {}

type Serializer<'a, 'b> = LowSerializer<Buffer<'b>, SubAllocator<'a>, Panic>;

fn serialize<'buf, F, T>(remote: &T, buf: &'buf mut [u8; 128]) -> Buffer<'buf>
where
    F: for<'a, 'b> SerializeWith<T, Serializer<'a, 'b>>,
{
    struct Wrap<'a, F, T>(&'a T, PhantomData<F>);

    impl<F, T> Archive for Wrap<'_, F, T>
    where
        F: ArchiveWith<T>,
    {
        type Archived = <F as ArchiveWith<T>>::Archived;
        type Resolver = <F as ArchiveWith<T>>::Resolver;

        fn resolve(
            &self,
            resolver: Self::Resolver,
            out: Place<Self::Archived>,
        ) {
            F::resolve_with(self.0, resolver, out)
        }
    }

    impl<'a, 'b, F, T> Serialize<Serializer<'a, 'b>> for Wrap<'_, F, T>
    where
        F: SerializeWith<T, Serializer<'a, 'b>>,
    {
        fn serialize(
            &self,
            serializer: &mut Serializer<'a, 'b>,
        ) -> Result<Self::Resolver, Panic> {
            F::serialize_with(self.0, serializer)
        }
    }

    let wrap = Wrap(remote, PhantomData::<F>);
    let writer = Buffer::from(buf);
    let mut scratch = [MaybeUninit::uninit(); 128];
    let alloc = SubAllocator::new(&mut scratch);

    rkyv::api::low::to_bytes_in_with_alloc::<_, _, Panic>(&wrap, writer, alloc)
        .always_ok()
}

fn access<F, T>(bytes: &[u8]) -> &<F as ArchiveWith<T>>::Archived
where
    F: ArchiveWith<T, Archived: CheckedArchived>,
{
    #[cfg(feature = "bytecheck")]
    {
        rkyv::api::low::access::<<F as ArchiveWith<T>>::Archived, Panic>(bytes)
            .always_ok()
    }

    #[cfg(not(feature = "bytecheck"))]
    unsafe {
        rkyv::access_unchecked::<<F as ArchiveWith<T>>::Archived>(bytes)
    }
}

#[derive(Debug, PartialEq)]
struct Foo([u8; 4]);

impl Default for Foo {
    fn default() -> Self {
        Self([2, 3, 5, 7])
    }
}

struct FooWrap;

impl ArchiveWith<Foo> for FooWrap {
    type Archived = Archived<[u8; 4]>;
    type Resolver = Resolver<[u8; 4]>;

    fn resolve_with(
        field: &Foo,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        field.0.resolve(resolver, out);
    }
}

impl<S> SerializeWith<Foo, S> for FooWrap
where
    S: Fallible<Error: Source> + Writer + ?Sized,
{
    fn serialize_with(
        field: &Foo,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field.0.serialize(serializer)
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<Archived<[u8; 4]>, Foo, D>
    for FooWrap
{
    fn deserialize_with(
        archived: &Archived<[u8; 4]>,
        deserializer: &mut D,
    ) -> Result<Foo, D::Error> {
        archived.deserialize(deserializer).map(Foo)
    }
}

struct Identity;

impl<T: Archive> ArchiveWith<T> for Identity {
    type Archived = Archived<T>;
    type Resolver = Resolver<T>;

    fn resolve_with(
        this: &T,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        this.resolve(resolver, out);
    }
}

impl<S: Fallible + ?Sized, T: Serialize<S>> SerializeWith<T, S> for Identity {
    fn serialize_with(
        this: &T,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        this.serialize(serializer)
    }
}

impl<D, T> DeserializeWith<Archived<T>, T, D> for Identity
where
    D: Fallible + ?Sized,
    T: Archive,
    Archived<T>: Deserialize<T, D>,
{
    fn deserialize_with(
        archived: &Archived<T>,
        deserializer: &mut D,
    ) -> Result<T, <D as Fallible>::Error> {
        archived.deserialize(deserializer)
    }
}
