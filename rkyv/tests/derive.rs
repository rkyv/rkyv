use std::{fmt::Debug, marker::PhantomData, path::PathBuf};

use rancor::{Fallible, Panic, ResultExt, Strategy};
use rkyv::{
    api::high::HighSerializer, ser::allocator::ArenaHandle, util::AlignedVec, with::{ArchiveWith, AsString, DeserializeWith, Map, SerializeWith}, Archive, Archived, Deserialize, Place, Resolver, Serialize
};

#[cfg(feature = "bytecheck")]
pub trait CheckedArchived: for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, Panic>> {}

#[cfg(feature = "bytecheck")]
impl<Archived: for<'a> rkyv::bytecheck::CheckBytes<rkyv::api::high::HighValidator<'a, Panic>>> CheckedArchived for Archived {}

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

fn access<F, T>(
    bytes: &[u8],
) -> &<F as ArchiveWith<T>>::Archived
where
    F: ArchiveWith<T, Archived: CheckedArchived>,
{
    #[cfg(feature = "bytecheck")]
    {
        rkyv::access::<<F as ArchiveWith<T>>::Archived, Panic>(bytes).always_ok()
    }

    #[cfg(not(feature = "bytecheck"))]
    unsafe {
        rkyv::access_unchecked::<<F as ArchiveWith<T>>::Archived>(bytes)
    }
}

fn roundtrip<F, T>(remote: &T)
where
    F: ArchiveWith<T, Archived: CheckedArchived>
        + for<'a> SerializeWith<T, Serializer<'a>>
        + DeserializeWith<
            <F as ArchiveWith<T>>::Archived,
            T,
            Strategy<(), Panic>,
        >,
    T: Debug + PartialEq,
{
    let bytes = serialize::<F, T>(remote);
    let archived = access::<F, T>(&bytes);
    let deserialized: T =
        F::deserialize_with(archived, Strategy::wrap(&mut ())).always_ok();

    assert_eq!(remote, &deserialized);
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

#[test]
fn named_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote<A> {
        a: u8,
        b: Vec<A>,
        c: Option<PathBuf>,
        d: Option<PathBuf>,
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<A>)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct Example<A> {
        a: u8,
        #[with(Identity)]
        b: Vec<A>,
        #[with(remote(Option<PathBuf>, with = Map<AsString>))]
        c: Option<String>,
        #[with(Map<Identity>, remote(Option<PathBuf>, with = Map<AsString>))]
        d: Option<String>,
    }

    let remote = Remote {
        a: 0,
        b: Vec::new(),
        c: Some("c".into()),
        d: Some("d".into())
    };

    roundtrip::<Example<i32>, _>(&remote);
}

#[test]
fn unnamed_struct() {
    #[derive(Debug, PartialEq)]
    struct Remote<A>(u8, Vec<A>, Option<PathBuf>, Option<PathBuf>);

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<A>)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct Example<A>(
        u8,
        #[with(Identity)] Vec<A>,
        #[with(remote(Option<PathBuf>, with = Map<AsString>))] Option<String>,
        #[with(Map<Identity>, remote(Option<PathBuf>, with = Map<AsString>))] Option<String>,
    );

    let remote = Remote(0, Vec::new(), Some("2".into()), Some("3".into()));
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
    enum Remote<A> {
        A,
        B(u8),
        C {
            a: Vec<A>,
            b: Option<PathBuf>,
            c: Option<PathBuf>,
        },
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = Remote::<A>)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    enum Example<A> {
        A,
        B(u8),
        C {
            #[with(Identity)]
            a: Vec<A>,
            #[with(remote(Option<PathBuf>, with = Map<AsString>))]
            b: Option<String>,
            #[with(Map<Identity>, remote(Option<PathBuf>, with = Map<AsString>))]
            c: Option<String>,
        },
    }

    for remote in [
        Remote::A,
        Remote::B(0),
        Remote::C {
            a: Vec::new(),
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
        #[derive(Copy, Clone, Default)]
        pub struct Remote {
            inner: [u8; 4],
        }

        impl Remote {
            pub fn new(inner: [u8; 4]) -> Self {
                Self { inner }
            }

            pub fn into_inner(self) -> [u8; 4] {
                self.inner
            }

            pub fn to_inner(&self) -> [u8; 4] {
                self.inner
            }

            pub fn as_inner(&self) -> &[u8; 4] {
                &self.inner
            }
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remove)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleByVal {
        #[with(remote(getter = remote::Remote::into_inner))]
        inner: [u8; 4],
    }

    impl From<ExampleByVal> for remote::Remote {
        fn from(value: ExampleByVal) -> Self {
            remote::Remote::new(value.inner)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleByRef {
        #[with(remote(getter = remote::Remote::to_inner))]
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
        #[with(remote(getter = remote::Remote::as_inner))]
        inner: [u8; 4],
    }

    impl From<ExampleThroughRef> for remote::Remote {
        fn from(value: ExampleThroughRef) -> Self {
            remote::Remote::new(value.inner)
        }
    }

    let remote = remote::Remote::default();
    roundtrip::<ExampleByRef, _>(&remote);
    roundtrip::<ExampleByRef, _>(&remote);
    roundtrip::<ExampleThroughRef, _>(&remote);
}

#[test]
fn unnamed_struct_private() {
    mod remote {
        #[derive(Copy, Clone, Default)]
        pub struct Remote([u8; 4]);

        impl Remote {
            pub fn new(inner: [u8; 4]) -> Self {
                Self(inner)
            }

            pub fn into_inner(self) -> [u8; 4] {
                self.0
            }

            pub fn as_inner(&self) -> [u8; 4] {
                self.0
            }
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleByRef(#[with(remote(getter = remote::Remote::as_inner))] [u8; 4]);

    impl From<ExampleByRef> for remote::Remote {
        fn from(value: ExampleByRef) -> Self {
            remote::Remote::new(value.0)
        }
    }

    #[derive(Archive, Serialize, Deserialize)]
    #[rkyv(remote = remote::Remote)]
    #[cfg_attr(feature = "bytecheck", rkyv(check_bytes))]
    struct ExampleByVal(
        #[with(remote(getter = remote::Remote::into_inner))] [u8; 4],
    );

    impl From<ExampleByVal> for remote::Remote {
        fn from(value: ExampleByVal) -> Self {
            remote::Remote::new(value.0)
        }
    }

    let remote = remote::Remote::default();
    roundtrip::<ExampleByRef, _>(&remote);
    roundtrip::<ExampleByVal, _>(&remote);
}
