use std::{fmt::Debug, marker::PhantomData, path::PathBuf};

use rancor::{Fallible, Panic, ResultExt, Strategy};
use rkyv::{
    api::high::HighSerializer,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
    with::{ArchiveWith, AsString, DeserializeWith, Map, SerializeWith},
    Archive, Archived, Deserialize, Place, Resolver, Serialize,
};

fn roundtrip<F, T>(remote: &T)
where
    F: ArchiveWith<T, Archived: CheckedArchived>
        + for<'a> SerializeWith<T, Serializer<'a>>
        + DeserializeWith< <F as ArchiveWith<T>>::Archived, T, Strategy<(),
          Panic>,
        >,
    T: Debug + PartialEq,
{
    let bytes = serialize::<F, T>(remote);
    let archived = access::<F, T>(&bytes);
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
        c: Option<PathBuf>,
        d: Option<PathBuf>,
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote<'a, A>)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct Example<'a, A> {
        a: u8,
        #[with(Identity, remote(with = Identity2))]
        b: PhantomData<&'a A>,
        #[with(remote(Option<PathBuf>, with = Map<AsString>))]
        c: Option<String>,
        #[with(Map<Identity>, remote(Option<PathBuf>, with = Map<AsString>))]
        d: Option<String>,
    }

    impl<'a, A> From<Example<'a, A>> for Remote<'a, A> {
        fn from(value: Example<'a, A>) -> Self {
            Remote {
                a: value.a,
                b: value.b,
                c: value.c.map(From::from),
                d: value.d.map(From::from),
            }
        }
    }

    let remote = Remote {
        a: 0,
        b: PhantomData,
        c: Some("c".into()),
        d: Some("d".into())
    };

    roundtrip::<Example<i32>, _>(&remote);
}

#[test]
fn unnamed_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote<'a, A>(u8, PhantomData<&'a A>, Option<PathBuf>, Option<PathBuf>);

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<'a, A>)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct Example<'a, A>(
        u8,
        #[with(Identity, remote(with = Identity2))] PhantomData<&'a A>,
        #[with(remote(Option<PathBuf>, with = Map<AsString>))] Option<String>,
        #[with(
            Map<Identity>,
            remote(Option<PathBuf>, with = Map<AsString>)
        )]
        Option<String>,
    );

    impl<'a, A> From<Example<'a, A>> for Remote<'a, A> {
        fn from(value: Example<'a, A>) -> Self {
            Remote(
                value.0,
                value.1,
                value.2.map(From::from),
                value.3.map(From::from),
            )
        }
    }

    let remote = Remote(0, PhantomData, Some("2".into()), Some("3".into()));
    roundtrip::<Example<i32>, _>(&remote);
}

#[test]
fn unit_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote;

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct Example;

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
            b: Option<PathBuf>,
            c: Option<PathBuf>,
        },
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<'a, A>)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    enum Example<'a, A> {
        A,
        B(u8),
        C {
            #[with(Identity, remote(with = Identity2))]
            a: PhantomData<&'a A>,
            #[with(remote(Option<PathBuf>, with = Map<AsString>))]
            b: Option<String>,
            #[with(
                Map<Identity>,
                remote(Option<PathBuf>, with = Map<AsString>)
            )]
            c: Option<String>,
        },
    }

    impl<'a, A> From<Example<'a, A>> for Remote<'a, A> {
        fn from(value: Example<'a, A>) -> Self {
            match value {
                Example::A => Remote::A,
                Example::B(value) => Remote::B(value),
                Example::C { a, b, c } => Remote::C {
                    a,
                    b: b.map(From::from),
                    c: c.map(From::from)
                },
            }
        }
    }

    for remote in [
        Remote::A,
        Remote::B(0),
        Remote::C {
            a: PhantomData,
            b: Some("b".into()),
            c: Some("c".into()),
        },
    ] {
        roundtrip::<Example<i32>, _>(&remote);
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
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleByRef {
        #[with(remote(getter = remote::Remote::inner))]
        inner: [u8; 4],
    }

    impl From<ExampleByRef> for remote::Remote {
        fn from(value: ExampleByRef) -> Self {
            remote::Remote::new(value.inner)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleThroughRef {
        #[with(remote(getter = remote::Remote::inner_ref))]
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
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleByRef(
        #[with(remote(getter = remote::Remote::inner))]
        [u8; 4]
    );

    impl From<ExampleByRef> for remote::Remote {
        fn from(value: ExampleByRef) -> Self {
            remote::Remote::new(value.0)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleThroughRef(
        #[with(remote(getter = remote::Remote::inner_ref))] [u8; 4],
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
    for<'a> rkyv::bytecheck::CheckBytes<
        rkyv::api::high::HighValidator<'a, Panic>
    >
{}

#[cfg(feature = "bytecheck")]
impl<Archived:
    for<'a> rkyv::bytecheck::CheckBytes<
        rkyv::api::high::HighValidator<'a, Panic>
    >
>
CheckedArchived for Archived {}

#[cfg(not(feature = "bytecheck"))]
pub trait CheckedArchived {}

#[cfg(not(feature = "bytecheck"))]
impl<Archived> CheckedArchived for Archived {}

type Serializer<'a> = HighSerializer<'a, AlignedVec, ArenaHandle<'a>, Panic>;

fn serialize<F, T>(remote: &T) -> AlignedVec
where
    F: for<'a> SerializeWith<T, Serializer<'a>>,
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

    impl<'a, F, T> Serialize<Serializer<'a>> for Wrap<'_, F, T>
    where
        F: SerializeWith<T, Serializer<'a>>,
    {
        fn serialize(
            &self,
            serializer: &mut Serializer<'a>,
        ) -> Result<Self::Resolver, Panic> {
            F::serialize_with(self.0, serializer)
        }
    }

    let wrap = Wrap(remote, PhantomData::<F>);

    rkyv::api::high::to_bytes::<Panic>(&wrap).always_ok()
}

fn access<F, T>(bytes: &[u8]) -> &<F as ArchiveWith<T>>::Archived
where
    F: ArchiveWith<T, Archived: CheckedArchived>,
{
    #[cfg(feature = "bytecheck")]
    {
        rkyv::access::<<F as ArchiveWith<T>>::Archived, Panic>(bytes)
            .always_ok()
    }

    #[cfg(not(feature = "bytecheck"))]
    unsafe {
        rkyv::access_unchecked::<<F as ArchiveWith<T>>::Archived>(bytes)
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
    Archived<T>: Deserialize<T, D>
{
    fn deserialize_with(archived: &Archived<T>, deserializer: &mut D)
        -> Result<T, <D as Fallible>::Error> {
            archived.deserialize(deserializer)
    }
}

struct Identity2;

impl<T: Archive> ArchiveWith<T> for Identity2 {
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

impl<S: Fallible + ?Sized, T: Serialize<S>> SerializeWith<T, S> for Identity2 {
    fn serialize_with(
        this: &T,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        this.serialize(serializer)
    }
}

impl<D, T> DeserializeWith<Archived<T>, T, D> for Identity2
where
    D: Fallible + ?Sized,
    T: Archive,
    Archived<T>: Deserialize<T, D>
{
    fn deserialize_with(archived: &Archived<T>, deserializer: &mut D)
        -> Result<T, <D as Fallible>::Error> {
            archived.deserialize(deserializer)
    }
}