use core::{marker::PhantomData, ops::ControlFlow};

use ptr_meta::Pointee;
use rancor::{Fallible, Source};

use crate::{
    alloc::{
        borrow::Cow,
        boxed::Box,
        collections::{BTreeMap, BTreeSet},
        rc::Rc,
        vec::Vec,
    },
    collections::{
        btree_map::{ArchivedBTreeMap, BTreeMapResolver},
        util::{Entry, EntryAdapter},
    },
    impls::core::with::RefWrapper,
    niche::option_box::{ArchivedOptionBox, OptionBoxResolver},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    traits::LayoutRaw,
    vec::{ArchivedVec, VecResolver},
    with::{
        ArchiveWith, AsOwned, AsVec, DeserializeWith, Map, MapKV, Niche,
        SerializeWith, Unshare,
    },
    Archive, ArchiveUnsized, ArchivedMetadata, Deserialize, DeserializeUnsized,
    Place, Serialize, SerializeUnsized,
};

// Implementation for `MapKV`

impl<A, B, K, V> ArchiveWith<BTreeMap<K, V>> for MapKV<A, B>
where
    A: ArchiveWith<K>,
    B: ArchiveWith<V>,
{
    type Archived = ArchivedBTreeMap<
        <A as ArchiveWith<K>>::Archived,
        <B as ArchiveWith<V>>::Archived,
    >;
    type Resolver = BTreeMapResolver;

    fn resolve_with(
        field: &BTreeMap<K, V>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedBTreeMap::resolve_from_len(field.len(), resolver, out)
    }
}

impl<A, B, K, V, S> SerializeWith<BTreeMap<K, V>, S> for MapKV<A, B>
where
    A: ArchiveWith<K> + SerializeWith<K, S>,
    B: ArchiveWith<V> + SerializeWith<V, S>,
    <A as ArchiveWith<K>>::Archived: Ord,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize_with(
        field: &BTreeMap<K, V>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedBTreeMap::<_, _, 5>::serialize_from_ordered_iter(
            field.iter().map(|(k, v)| {
                (
                    RefWrapper::<'_, A, K>(k, PhantomData::<A>),
                    RefWrapper::<'_, B, V>(v, PhantomData::<B>),
                )
            }),
            serializer,
        )
    }
}

impl<A, B, K, V, D>
    DeserializeWith<
        ArchivedBTreeMap<
            <A as ArchiveWith<K>>::Archived,
            <B as ArchiveWith<V>>::Archived,
        >,
        BTreeMap<K, V>,
        D,
    > for MapKV<A, B>
where
    A: ArchiveWith<K> + DeserializeWith<<A as ArchiveWith<K>>::Archived, K, D>,
    B: ArchiveWith<V> + DeserializeWith<<B as ArchiveWith<V>>::Archived, V, D>,
    K: Ord,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedBTreeMap<
            <A as ArchiveWith<K>>::Archived,
            <B as ArchiveWith<V>>::Archived,
        >,
        deserializer: &mut D,
    ) -> Result<BTreeMap<K, V>, <D as Fallible>::Error> {
        let mut result = BTreeMap::new();
        let r = field.visit(|ak, av| {
            let k = match A::deserialize_with(ak, deserializer) {
                Ok(k) => k,
                Err(e) => return ControlFlow::Break(e),
            };
            let v = match B::deserialize_with(av, deserializer) {
                Ok(v) => v,
                Err(e) => return ControlFlow::Break(e),
            };
            result.insert(k, v);
            ControlFlow::Continue(())
        });
        match r {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

// Implementations for `Map`
impl<A, O> ArchiveWith<Vec<O>> for Map<A>
where
    A: ArchiveWith<O>,
{
    type Archived = ArchivedVec<<A as ArchiveWith<O>>::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &Vec<O>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out)
    }
}

impl<A, O, S> SerializeWith<Vec<O>, S> for Map<A>
where
    S: Fallible + Allocator + Writer + ?Sized,
    A: ArchiveWith<O> + SerializeWith<O, S>,
{
    fn serialize_with(
        field: &Vec<O>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        // Wrapper for O so that we have an Archive and Serialize implementation
        // and ArchivedVec::serialize_from_* is happy about the bound
        // constraints
        struct RefWrapper<'o, A, O>(&'o O, PhantomData<A>);

        impl<A: ArchiveWith<O>, O> Archive for RefWrapper<'_, A, O> {
            type Archived = <A as ArchiveWith<O>>::Archived;
            type Resolver = <A as ArchiveWith<O>>::Resolver;

            fn resolve(
                &self,
                resolver: Self::Resolver,
                out: Place<Self::Archived>,
            ) {
                A::resolve_with(self.0, resolver, out)
            }
        }

        impl<A, O, S> Serialize<S> for RefWrapper<'_, A, O>
        where
            A: ArchiveWith<O> + SerializeWith<O, S>,
            S: Fallible + Writer + ?Sized,
        {
            fn serialize(&self, s: &mut S) -> Result<Self::Resolver, S::Error> {
                A::serialize_with(self.0, s)
            }
        }

        let iter = field
            .iter()
            .map(|value| RefWrapper::<'_, A, O>(value, PhantomData));

        ArchivedVec::serialize_from_iter(iter, s)
    }
}

impl<A, O, D>
    DeserializeWith<ArchivedVec<<A as ArchiveWith<O>>::Archived>, Vec<O>, D>
    for Map<A>
where
    A: ArchiveWith<O> + DeserializeWith<<A as ArchiveWith<O>>::Archived, O, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<<A as ArchiveWith<O>>::Archived>,
        d: &mut D,
    ) -> Result<Vec<O>, D::Error> {
        field
            .iter()
            .map(|value| A::deserialize_with(value, d))
            .collect()
    }
}

// AsOwned

impl<'a, F: Archive + Clone> ArchiveWith<Cow<'a, F>> for AsOwned {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &Cow<'a, F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        field.resolve(resolver, out);
    }
}

impl<'a, F, S> SerializeWith<Cow<'a, F>, S> for AsOwned
where
    F: Serialize<S> + Clone,
    S: Fallible + ?Sized,
{
    fn serialize_with(
        field: &Cow<'a, F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field.serialize(serializer)
    }
}

impl<T, D> DeserializeWith<T::Archived, T, D> for AsOwned
where
    T: Archive + Clone,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &T::Archived,
        deserializer: &mut D,
    ) -> Result<T, D::Error> {
        field.deserialize(deserializer)
    }
}

impl<'a, T: Archive + Clone> ArchiveWith<Cow<'a, [T]>> for AsOwned {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &Cow<'a, [T]>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_slice(field, resolver, out);
    }
}

impl<'a, T, S> SerializeWith<Cow<'a, [T]>, S> for AsOwned
where
    T: Serialize<S> + Clone,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &Cow<'a, [T]>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(field, serializer)
    }
}

impl<'a, T, D> DeserializeWith<ArchivedVec<T::Archived>, Cow<'a, [T]>, D>
    for AsOwned
where
    T: Archive + Clone,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<Cow<'a, [T]>, D::Error> {
        Ok(Cow::Owned(field.deserialize(deserializer)?))
    }
}

impl<'a> ArchiveWith<Cow<'a, str>> for AsOwned {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    fn resolve_with(
        field: &Cow<'a, str>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedString::resolve_from_str(field, resolver, out);
    }
}

impl<'a, S> SerializeWith<Cow<'a, str>, S> for AsOwned
where
    S: Fallible + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize_with(
        field: &Cow<'a, str>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(field, serializer)
    }
}

impl<'a, D> DeserializeWith<ArchivedString, Cow<'a, str>, D> for AsOwned
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedString,
        deserializer: &mut D,
    ) -> Result<Cow<'a, str>, D::Error> {
        Ok(Cow::Owned(field.deserialize(deserializer)?))
    }
}

// AsVec

impl<K: Archive, V: Archive> ArchiveWith<BTreeMap<K, V>> for AsVec {
    type Archived = ArchivedVec<Entry<K::Archived, V::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &BTreeMap<K, V>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<K, V, S> SerializeWith<BTreeMap<K, V>, S> for AsVec
where
    K: Serialize<S>,
    V: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &BTreeMap<K, V>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_iter(
            field.iter().map(|(key, value)| {
                EntryAdapter::<_, _, K, V>::new(key, value)
            }),
            serializer,
        )
    }
}

impl<K, V, D>
    DeserializeWith<
        ArchivedVec<Entry<K::Archived, V::Archived>>,
        BTreeMap<K, V>,
        D,
    > for AsVec
where
    K: Archive + Ord,
    V: Archive,
    K::Archived: Deserialize<K, D>,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<Entry<K::Archived, V::Archived>>,
        deserializer: &mut D,
    ) -> Result<BTreeMap<K, V>, D::Error> {
        let mut result = BTreeMap::new();
        for entry in field.iter() {
            result.insert(
                entry.key.deserialize(deserializer)?,
                entry.value.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<T: Archive> ArchiveWith<BTreeSet<T>> for AsVec {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &BTreeSet<T>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<T, S> SerializeWith<BTreeSet<T>, S> for AsVec
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &BTreeSet<T>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::<T::Archived>::serialize_from_iter::<T, _, _>(
            field.iter(),
            serializer,
        )
    }
}

impl<T, D> DeserializeWith<ArchivedVec<T::Archived>, BTreeSet<T>, D> for AsVec
where
    T: Archive + Ord,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<BTreeSet<T>, D::Error> {
        let mut result = BTreeSet::new();
        for key in field.iter() {
            result.insert(key.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

// Niche

impl<T> ArchiveWith<Option<Box<T>>> for Niche
where
    T: ArchiveUnsized + ?Sized,
    ArchivedMetadata<T>: Default,
{
    type Archived = ArchivedOptionBox<T::Archived>;
    type Resolver = OptionBoxResolver;

    fn resolve_with(
        field: &Option<Box<T>>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedOptionBox::resolve_from_option(field.as_deref(), resolver, out);
    }
}

impl<T, S> SerializeWith<Option<Box<T>>, S> for Niche
where
    T: SerializeUnsized<S> + ?Sized,
    S: Fallible + Writer + ?Sized,
    ArchivedMetadata<T>: Default,
{
    fn serialize_with(
        field: &Option<Box<T>>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedOptionBox::serialize_from_option(field.as_deref(), serializer)
    }
}

impl<T, D> DeserializeWith<ArchivedOptionBox<T::Archived>, Option<Box<T>>, D>
    for Niche
where
    T: ArchiveUnsized + LayoutRaw + Pointee + ?Sized,
    T::Archived: DeserializeUnsized<T, D>,
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        field: &ArchivedOptionBox<T::Archived>,
        deserializer: &mut D,
    ) -> Result<Option<Box<T>>, D::Error> {
        if let Some(value) = field.as_ref() {
            Ok(Some(value.deserialize(deserializer)?))
        } else {
            Ok(None)
        }
    }
}

// Unshare

#[cfg(target_has_atomic = "ptr")]
impl<T: Archive> ArchiveWith<crate::alloc::sync::Arc<T>> for Unshare {
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    fn resolve_with(
        x: &crate::alloc::sync::Arc<T>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        x.as_ref().resolve(resolver, out)
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<T, S> SerializeWith<crate::alloc::sync::Arc<T>, S> for Unshare
where
    T: Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize_with(
        x: &crate::alloc::sync::Arc<T>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        x.as_ref().serialize(s)
    }
}

#[cfg(target_has_atomic = "ptr")]
impl<A, T, D> DeserializeWith<A, crate::alloc::sync::Arc<T>, D> for Unshare
where
    A: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        x: &A,
        d: &mut D,
    ) -> Result<crate::alloc::sync::Arc<T>, D::Error> {
        Ok(crate::alloc::sync::Arc::new(A::deserialize(x, d)?))
    }
}

impl<T: Archive> ArchiveWith<Rc<T>> for Unshare {
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    fn resolve_with(
        x: &Rc<T>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        x.as_ref().resolve(resolver, out)
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> SerializeWith<Rc<T>, S>
    for Unshare
{
    fn serialize_with(
        x: &Rc<T>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        x.as_ref().serialize(s)
    }
}

impl<A, T, D> DeserializeWith<A, Rc<T>, D> for Unshare
where
    A: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(x: &A, d: &mut D) -> Result<Rc<T>, D::Error> {
        Ok(Rc::new(A::deserialize(x, d)?))
    }
}

#[cfg(test)]
mod tests {
    use core::mem::size_of;

    use crate::{
        alloc::{
            borrow::Cow,
            boxed::Box,
            collections::{BTreeMap, BTreeSet},
            string::{String, ToString},
        },
        api::test::{roundtrip, roundtrip_with, to_archived},
        niche::niching::Null,
        with::{
            AsOwned, AsVec, DefaultNiche, InlineAsBox, Map, MapKV, Niche,
            NicheInto,
        },
        Archive, Deserialize, Serialize,
    };

    #[derive(Debug, Archive, Deserialize, Serialize, PartialEq)]
    #[rkyv(crate, compare(PartialEq), derive(Debug))]
    struct Test {
        value: Option<Box<u128>>,
    }

    #[test]
    fn roundtrip_niche_none() {
        roundtrip(&Test { value: None });
    }

    #[test]
    fn roundtrip_niche_some() {
        roundtrip(&Test {
            value: Some(Box::new(128)),
        });
    }

    #[test]
    fn ambiguous_niched_archived_box() {
        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[rkyv(crate, compare(PartialEq), derive(Debug))]
        struct HasNiche {
            #[rkyv(with = Niche)]
            inner: Option<Box<[u32]>>,
        }

        roundtrip(&HasNiche {
            inner: Some(Box::<[u32]>::from([])),
        });
        roundtrip(&HasNiche { inner: None });
    }

    #[test]
    fn with_as_owned() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[rkyv(with = AsOwned)]
            a: Cow<'a, u32>,
            #[rkyv(with = AsOwned)]
            b: Cow<'a, [u32]>,
            #[rkyv(with = AsOwned)]
            c: Cow<'a, str>,
        }

        let value = Test {
            a: Cow::Borrowed(&100),
            b: Cow::Borrowed(&[1, 2, 3, 4, 5, 6]),
            c: Cow::Borrowed("hello world"),
        };
        to_archived(&value, |archived| {
            assert_eq!(archived.a, 100);
            assert_eq!(archived.b, [1, 2, 3, 4, 5, 6]);
            assert_eq!(archived.c, "hello world");
        });
    }

    #[test]
    fn with_as_map() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[rkyv(with = Map<InlineAsBox>)]
            a: Option<&'a str>,
            #[rkyv(with = Map<InlineAsBox>)]
            b: Option<&'a str>,
        }

        let value = Test {
            a: Some("foo"),
            b: None,
        };

        to_archived(&value, |archived| {
            assert!(archived.a.is_some());
            assert!(archived.b.is_none());
        });
    }

    #[test]
    fn with_as_mapkv() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[rkyv(with = MapKV<InlineAsBox, InlineAsBox>)]
            a: BTreeMap<&'a str, &'a str>,
        }

        let mut a = BTreeMap::new();
        a.insert("foo", "bar");
        a.insert("woo", "roo");

        let value = Test { a };

        to_archived(&value, |archived| {
            assert_eq!(archived.a.len(), 2);
            assert!(archived.a.contains_key("foo"));
            assert_eq!(**archived.a.get("woo").unwrap(), *"roo");
        });
    }

    #[test]
    fn with_as_vec() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[rkyv(with = AsVec)]
            a: BTreeMap<String, String>,
            #[rkyv(with = AsVec)]
            b: BTreeSet<String>,
            #[rkyv(with = AsVec)]
            c: BTreeMap<String, String>,
        }

        let mut a = BTreeMap::new();
        a.insert("foo".to_string(), "hello".to_string());
        a.insert("bar".to_string(), "world".to_string());
        a.insert("baz".to_string(), "bat".to_string());

        let mut b = BTreeSet::new();
        b.insert("foo".to_string());
        b.insert("hello world!".to_string());
        b.insert("bar".to_string());
        b.insert("fizzbuzz".to_string());

        let c = BTreeMap::new();

        let value = Test { a, b, c };

        to_archived(&value, |archived| {
            assert_eq!(archived.a.len(), 3);
            assert!(archived
                .a
                .iter()
                .find(|&e| e.key == "foo" && e.value == "hello")
                .is_some());
            assert!(archived
                .a
                .iter()
                .find(|&e| e.key == "bar" && e.value == "world")
                .is_some());
            assert!(archived
                .a
                .iter()
                .find(|&e| e.key == "baz" && e.value == "bat")
                .is_some());

            assert_eq!(archived.b.len(), 4);
            assert!(archived.b.iter().find(|&e| e == "foo").is_some());
            assert!(archived.b.iter().find(|&e| e == "hello world!").is_some());
            assert!(archived.b.iter().find(|&e| e == "bar").is_some());
            assert!(archived.b.iter().find(|&e| e == "fizzbuzz").is_some());
        });
    }

    #[cfg(feature = "alloc")]
    #[test]
    fn with_niche_box() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct TestNiche {
            #[rkyv(with = Niche)]
            inner: Option<Box<String>>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct TestNullNiche {
            #[rkyv(with = NicheInto<Null>)]
            inner: Option<Box<String>>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct TestNoNiching {
            inner: Option<Box<String>>,
        }

        let value = TestNiche {
            inner: Some(Box::new("hello world".to_string())),
        };
        to_archived(&value, |archived| {
            assert!(archived.inner.is_some());
            assert_eq!(&**archived.inner.as_ref().unwrap(), "hello world");
            assert_eq!(archived.inner, value.inner);
        });

        let value = TestNiche { inner: None };
        to_archived(&value, |archived| {
            assert!(archived.inner.is_none());
            assert_eq!(archived.inner, value.inner);
        });
        assert!(
            size_of::<ArchivedTestNiche>() < size_of::<ArchivedTestNoNiching>()
        );

        let value = TestNullNiche {
            inner: Some(Box::new("hello world".to_string())),
        };
        to_archived(&value, |archived| {
            assert!(archived.inner.is_some());
            assert_eq!(&**archived.inner.as_ref().unwrap(), "hello world");
            assert_eq!(archived.inner, value.inner);
        });

        let value = TestNullNiche { inner: None };
        to_archived(&value, |archived| {
            assert!(archived.inner.is_none());
            assert_eq!(archived.inner, value.inner);
        });
        assert!(
            size_of::<ArchivedTestNullNiche>()
                < size_of::<ArchivedTestNoNiching>()
        );
    }

    #[test]
    fn with_null_niching() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Nichable {
            #[rkyv(niche)] // Default = Null
            boxed: Box<i32>,
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = DefaultNiche)]
            field: Option<Nichable>,
        }

        assert_eq!(size_of::<ArchivedNichable>(), size_of::<ArchivedOuter>());

        let values = [
            Outer { field: None },
            Outer {
                field: Some(Nichable {
                    boxed: Box::new(727),
                }),
            },
        ];

        roundtrip_with(&values[0], |_, archived| {
            assert!(archived.field.is_none());
        });
        roundtrip_with(&values[1], |_, archived| {
            let nichable = archived.field.as_ref().unwrap();
            assert_eq!(nichable.boxed.as_ref().to_native(), 727);
        });
    }
}
