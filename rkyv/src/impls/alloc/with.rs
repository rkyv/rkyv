use core::{iter, marker::PhantomData};

use ptr_meta::Pointee;
use rancor::{Fallible, Source};

use crate::{
    alloc::{
        borrow::Cow,
        boxed::Box,
        collections::{BTreeMap, BTreeSet},
        rc::Rc,
        sync::Arc,
        vec::Vec,
    }, collections::util::{Entry, EntryAdapter, EntryAdapterWith, EntryResolver}, niche::option_box::{ArchivedOptionBox, OptionBoxResolver}, ser::{Allocator, Writer}, string::{ArchivedString, StringResolver}, traits::LayoutRaw, util::AlignedVec, vec::{ArchivedVec, VecResolver}, with::{
        ArchiveWith, AsOwned, AsVec, DeserializeWith, Map, MapKV, Niche, SerializeWith, Unshare
    }, Archive, ArchiveUnsized, ArchivedMetadata, Deserialize, DeserializeUnsized, Place, Serialize, SerializeUnsized
};




// Implementations for `MapKV`
impl<A: ArchiveWith<K>, B: ArchiveWith<V>, K, V> ArchiveWith<BTreeMap<K, V>> for MapKV<A, B>
{
    type Archived = ArchivedVec<Entry<<A as ArchiveWith<K>>::Archived, <B as ArchiveWith<V>>::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
            field: &BTreeMap<K, V>,
            resolver: Self::Resolver,
            out: Place<Self::Archived>,
        ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<A, B, K, V, S> SerializeWith<BTreeMap<K, V>, S> for MapKV<A, B>
where
    A: ArchiveWith<K> + SerializeWith<K, S>,
    B: ArchiveWith<V> + SerializeWith<V, S>,
   // K: Serialize + Archive,
    //V: Serialize + Archive,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
            field: &BTreeMap<K, V>,
            serializer: &mut S,
        ) -> Result<Self::Resolver, <S as Fallible>::Error> {

       
        ArchivedVec::serialize_from_iter(field.iter()
            .map(|(key, value)| {
                /*
                EntryAdapter {
                    key: &A::serialize_with(key, serializer)?,
                    value: &B::serialize_with(value, serializer)?
                }
                */

                EntryAdapterWith {
                    key,
                    value,
                    _keyser: PhantomData::<A>,
                    _valser: PhantomData::<B>
                }
            })
        , serializer)
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
            field.iter().map(|(key, value)| EntryAdapter { key, value }),
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

impl<T: Archive> ArchiveWith<Arc<T>> for Unshare {
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    fn resolve_with(
        x: &Arc<T>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        x.as_ref().resolve(resolver, out)
    }
}

impl<T, S> SerializeWith<Arc<T>, S> for Unshare
where
    T: Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize_with(
        x: &Arc<T>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        x.as_ref().serialize(s)
    }
}

impl<A, T, D> DeserializeWith<A, Arc<T>, D> for Unshare
where
    A: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(x: &A, d: &mut D) -> Result<Arc<T>, D::Error> {
        Ok(Arc::new(A::deserialize(x, d)?))
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
        }, api::test::{roundtrip, to_archived}, boxed::ArchivedBox, option::ArchivedOption, string::ArchivedString, with::{AsOwned, AsVec, Inline, InlineAsBox, Map, MapKV, Niche}, Archive, Deserialize, Serialize
    };

    #[derive(Debug, Archive, Deserialize, Serialize, PartialEq)]
    #[rkyv(crate, check_bytes, compare(PartialEq), derive(Debug))]
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
        #[rkyv(crate, check_bytes, compare(PartialEq), derive(Debug))]
        struct HasNiche {
            #[with(Niche)]
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
        #[rkyv(crate, check_bytes)]
        struct Test<'a> {
            #[with(AsOwned)]
            a: Cow<'a, u32>,
            #[with(AsOwned)]
            b: Cow<'a, [u32]>,
            #[with(AsOwned)]
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
        #[rkyv(crate, check_bytes)]
        struct Test<'a> {
            #[with(Map<InlineAsBox>)]
            a: Option<&'a str>,
            #[with(Map<InlineAsBox>)]
            b: Option<&'a str>,
        }

        let value = Test {
            a: Some("foo"),
            b: None
        };


        to_archived(&value, |archived| {
            assert!(archived.a.is_some());
            assert!(archived.b.is_none());
        });

    }

    #[test]
    fn with_as_mapkv() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate, check_bytes)]
        struct Test<'a> {
            //#[with(MapKV<InlineAsBox, InlineAsBox>)]
            #[with(MapKV<InlineAsBox, InlineAsBox>)]
            a: BTreeMap<&'a str, &'a str>
        }


        let mut a = BTreeMap::new();
        a.insert("foo", "bar");

        let value = Test {
          a,   
            //   a: Some("foo")
        };



        to_archived(&value, |archived| {
          //  assert_eq!(archived.a.len(), 1);
        });

    }

    #[test]
    fn with_as_vec() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate, check_bytes)]
        struct Test {
            #[with(AsVec)]
            a: BTreeMap<String, String>,
            #[with(AsVec)]
            b: BTreeSet<String>,
            #[with(AsVec)]
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
        #[rkyv(crate, check_bytes)]
        struct Test {
            #[with(Niche)]
            inner: Option<Box<String>>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate, check_bytes)]
        struct TestNoNiching {
            inner: Option<Box<String>>,
        }

        let value = Test {
            inner: Some(Box::new("hello world".to_string())),
        };
        to_archived(&value, |archived| {
            assert!(archived.inner.is_some());
            assert_eq!(&**archived.inner.as_ref().unwrap(), "hello world");
            assert_eq!(archived.inner, value.inner);
        });

        let value = Test { inner: None };
        to_archived(&value, |archived| {
            assert!(archived.inner.is_none());
            assert_eq!(archived.inner, value.inner);
        });
        assert!(size_of::<ArchivedTest>() < size_of::<ArchivedTestNoNiching>());
    }
}
