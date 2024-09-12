use core::{error::Error, fmt, hash::BuildHasher};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::{CStr, OsString},
    hash::Hash,
    marker::PhantomData,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Mutex, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use rancor::{Fallible, OptionExt, ResultExt, Source};

use crate::{
    collections::{
        swiss_table::{ArchivedHashMap, HashMapResolver},
        util::{Entry, EntryAdapter},
    },
    ffi::{ArchivedCString, CStringResolver},
    hash::FxHasher64,
    impls::core::with::RefWrapper,
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    time::ArchivedDuration,
    vec::{ArchivedVec, VecResolver},
    with::{
        ArchiveWith, AsOwned, AsString, AsUnixTime, AsVec, DeserializeWith,
        Lock, MapKV, SerializeWith,
    },
    Archive, Deserialize, Place, Serialize, SerializeUnsized,
};

// MapKV
impl<A, B, K, V, H> ArchiveWith<HashMap<K, V, H>> for MapKV<A, B>
where
    A: ArchiveWith<K>,
    B: ArchiveWith<V>,
    H: Default + BuildHasher,
{
    type Archived = ArchivedHashMap<
        <A as ArchiveWith<K>>::Archived,
        <B as ArchiveWith<V>>::Archived,
    >;
    type Resolver = HashMapResolver;

    fn resolve_with(
        field: &HashMap<K, V, H>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedHashMap::resolve_from_len(field.len(), (7, 8), resolver, out)
    }
}

impl<A, B, K, V, S, H> SerializeWith<HashMap<K, V, H>, S> for MapKV<A, B>
where
    A: ArchiveWith<K> + SerializeWith<K, S>,
    B: ArchiveWith<V> + SerializeWith<V, S>,
    K: Hash + Eq,
    <A as ArchiveWith<K>>::Archived: Eq + Hash,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
    H: Default + BuildHasher,
    H::Hasher: Default,
{
    fn serialize_with(
        field: &HashMap<K, V, H>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        ArchivedHashMap::<_, _, FxHasher64>::serialize_from_iter(
            field.iter().map(|(k, v)| {
                (
                    RefWrapper::<'_, A, K>(k, PhantomData::<A>),
                    RefWrapper::<'_, B, V>(v, PhantomData::<B>),
                )
            }),
            (7, 8),
            serializer,
        )
    }
}

impl<A, B, K, V, D, S>
    DeserializeWith<
        ArchivedHashMap<
            <A as ArchiveWith<K>>::Archived,
            <B as ArchiveWith<V>>::Archived,
        >,
        HashMap<K, V, S>,
        D,
    > for MapKV<A, B>
where
    A: ArchiveWith<K> + DeserializeWith<<A as ArchiveWith<K>>::Archived, K, D>,
    B: ArchiveWith<V> + DeserializeWith<<B as ArchiveWith<V>>::Archived, V, D>,
    K: Ord + Hash + Eq,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    fn deserialize_with(
        field: &ArchivedHashMap<
            <A as ArchiveWith<K>>::Archived,
            <B as ArchiveWith<V>>::Archived,
        >,
        deserializer: &mut D,
    ) -> Result<HashMap<K, V, S>, <D as Fallible>::Error> {
        let mut result =
            HashMap::with_capacity_and_hasher(field.len(), S::default());
        for (k, v) in field.iter() {
            result.insert(
                A::deserialize_with(k, deserializer)?,
                B::deserialize_with(v, deserializer)?,
            );
        }
        Ok(result)
    }
}

// AsString

#[derive(Debug)]
struct InvalidUtf8;

impl fmt::Display for InvalidUtf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8")
    }
}

impl Error for InvalidUtf8 {}

impl ArchiveWith<OsString> for AsString {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve_with(
        field: &OsString,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // It's safe to unwrap here because if the OsString wasn't valid UTF-8
        // it would have failed to serialize
        ArchivedString::resolve_from_str(
            field.to_str().unwrap(),
            resolver,
            out,
        );
    }
}

impl<S> SerializeWith<OsString, S> for AsString
where
    S: Fallible + ?Sized,
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize_with(
        field: &OsString,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(
            field.to_str().into_trace(InvalidUtf8)?,
            serializer,
        )
    }
}

impl<D> DeserializeWith<ArchivedString, OsString, D> for AsString
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedString,
        _: &mut D,
    ) -> Result<OsString, D::Error> {
        Ok(OsString::from_str(field.as_str()).unwrap())
    }
}

impl ArchiveWith<PathBuf> for AsString {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve_with(
        field: &PathBuf,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // It's safe to unwrap here because if the OsString wasn't valid UTF-8
        // it would have failed to serialize
        ArchivedString::resolve_from_str(
            field.to_str().unwrap(),
            resolver,
            out,
        );
    }
}

impl<S> SerializeWith<PathBuf, S> for AsString
where
    S: Fallible + ?Sized,
    S::Error: Source,
    str: SerializeUnsized<S>,
{
    fn serialize_with(
        field: &PathBuf,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(
            field.to_str().into_trace(InvalidUtf8)?,
            serializer,
        )
    }
}

impl<D> DeserializeWith<ArchivedString, PathBuf, D> for AsString
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedString,
        _: &mut D,
    ) -> Result<PathBuf, D::Error> {
        Ok(Path::new(field.as_str()).to_path_buf())
    }
}

// Lock

#[derive(Debug)]
struct LockPoisoned;

impl fmt::Display for LockPoisoned {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lock poisoned")
    }
}

impl Error for LockPoisoned {}

impl<F: Archive> ArchiveWith<Mutex<F>> for Lock {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &Mutex<F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // Unfortunately, we have to unwrap here because resolve must be
        // infallible
        //
        // An alternative would be to only implement ArchiveWith for
        // Arc<Mutex<F>>, copy an Arc into the resolver, and hold the
        // guard in there as well (as a reference to the internal mutex).
        // This unfortunately will cause a deadlock if two Arcs to the same
        // Mutex are serialized before the first is resolved. The
        // compromise is, unfortunately, to just unwrap poison
        // errors here and document it.
        field.lock().unwrap().resolve(resolver, out);
    }
}

impl<F, S> SerializeWith<Mutex<F>, S> for Lock
where
    F: Serialize<S>,
    S: Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize_with(
        field: &Mutex<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field
            .lock()
            .ok()
            .into_trace(LockPoisoned)?
            .serialize(serializer)
    }
}

impl<F, T, D> DeserializeWith<F, Mutex<T>, D> for Lock
where
    F: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &F,
        deserializer: &mut D,
    ) -> Result<Mutex<T>, D::Error> {
        Ok(Mutex::new(field.deserialize(deserializer)?))
    }
}

impl<F: Archive> ArchiveWith<RwLock<F>> for Lock {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &RwLock<F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // Unfortunately, we have to unwrap here because resolve must be
        // infallible
        //
        // An alternative would be to only implement ArchiveWith for
        // Arc<Mutex<F>>, copy a an Arc into the resolver, and hold the
        // guard in there as well (as a reference to the internal
        // mutex). This unfortunately will cause a deadlock if two Arcs to the
        // same Mutex are serialized before the first is resolved. The
        // compromise is, unfortunately, to just unwrap poison errors
        // here and document it.
        field.read().unwrap().resolve(resolver, out);
    }
}

impl<F, S> SerializeWith<RwLock<F>, S> for Lock
where
    F: Serialize<S>,
    S: Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize_with(
        field: &RwLock<F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field
            .read()
            .ok()
            .into_trace(LockPoisoned)?
            .serialize(serializer)
    }
}

impl<F, T, D> DeserializeWith<F, RwLock<T>, D> for Lock
where
    F: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &F,
        deserializer: &mut D,
    ) -> Result<RwLock<T>, D::Error> {
        Ok(RwLock::new(field.deserialize(deserializer)?))
    }
}

// AsVec

impl<K: Archive, V: Archive, H> ArchiveWith<HashMap<K, V, H>> for AsVec {
    type Archived = ArchivedVec<Entry<K::Archived, V::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &HashMap<K, V, H>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<K, V, H, S> SerializeWith<HashMap<K, V, H>, S> for AsVec
where
    K: Serialize<S>,
    V: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &HashMap<K, V, H>,
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

impl<K, V, H, D>
    DeserializeWith<
        ArchivedVec<Entry<K::Archived, V::Archived>>,
        HashMap<K, V, H>,
        D,
    > for AsVec
where
    K: Archive + Hash + Eq,
    V: Archive,
    K::Archived: Deserialize<K, D>,
    V::Archived: Deserialize<V, D>,
    H: BuildHasher + Default,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<Entry<K::Archived, V::Archived>>,
        deserializer: &mut D,
    ) -> Result<HashMap<K, V, H>, D::Error> {
        let mut result =
            HashMap::with_capacity_and_hasher(field.len(), H::default());
        for entry in field.iter() {
            result.insert(
                entry.key.deserialize(deserializer)?,
                entry.value.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<T: Archive, H> ArchiveWith<HashSet<T, H>> for AsVec {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &HashSet<T, H>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<T, H, S> SerializeWith<HashSet<T, H>, S> for AsVec
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &HashSet<T, H>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::<T::Archived>::serialize_from_iter::<T, _, _>(
            field.iter(),
            serializer,
        )
    }
}

impl<T, H, D> DeserializeWith<ArchivedVec<T::Archived>, HashSet<T, H>, D>
    for AsVec
where
    T: Archive + Hash + Eq,
    T::Archived: Deserialize<T, D>,
    H: BuildHasher + Default,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<HashSet<T, H>, D::Error> {
        let mut result =
            HashSet::with_capacity_and_hasher(field.len(), H::default());
        for key in field.iter() {
            result.insert(key.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

// UnixTimestamp

impl ArchiveWith<SystemTime> for AsUnixTime {
    type Archived = ArchivedDuration;
    type Resolver = ();

    #[inline]
    fn resolve_with(
        field: &SystemTime,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        // We already checked the duration during serialize_with
        let duration = field.duration_since(UNIX_EPOCH).unwrap();
        Archive::resolve(&duration, resolver, out);
    }
}

impl<S> SerializeWith<SystemTime, S> for AsUnixTime
where
    S: Fallible + ?Sized,
    S::Error: Source,
{
    fn serialize_with(
        field: &SystemTime,
        _: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field.duration_since(UNIX_EPOCH).into_error()?;
        Ok(())
    }
}

impl<D> DeserializeWith<ArchivedDuration, SystemTime, D> for AsUnixTime
where
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedDuration,
        _: &mut D,
    ) -> Result<SystemTime, D::Error> {
        // `checked_add` forces correct type deduction when multiple `Duration`
        // are present.
        Ok(UNIX_EPOCH.checked_add((*field).into()).unwrap())
    }
}

// AsOwned

impl<'a> ArchiveWith<Cow<'a, CStr>> for AsOwned {
    type Archived = ArchivedCString;
    type Resolver = CStringResolver;

    #[inline]
    fn resolve_with(
        field: &Cow<'a, CStr>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedCString::resolve_from_c_str(field, resolver, out);
    }
}

impl<'a, S> SerializeWith<Cow<'a, CStr>, S> for AsOwned
where
    S: Fallible + Writer + ?Sized,
{
    fn serialize_with(
        field: &Cow<'a, CStr>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedCString::serialize_from_c_str(field, serializer)
    }
}

impl<'a, D> DeserializeWith<ArchivedCString, Cow<'a, CStr>, D> for AsOwned
where
    D: Fallible + ?Sized,
    D::Error: Source,
{
    fn deserialize_with(
        field: &ArchivedCString,
        deserializer: &mut D,
    ) -> Result<Cow<'a, CStr>, D::Error> {
        Ok(Cow::Owned(field.deserialize(deserializer)?))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::BTreeMap,
        ffi::OsString,
        path::PathBuf,
        sync::{Mutex, RwLock},
    };

    use crate::{
        alloc::collections::HashMap,
        api::test::{roundtrip_with, to_archived},
        with::{AsString, InlineAsBox, Lock, MapKV},
        Archive, Deserialize, Serialize,
    };

    #[test]
    fn roundtrip_mutex() {
        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[rkyv(crate, derive(Debug, PartialEq))]
        struct Test {
            #[rkyv(with = Lock)]
            value: Mutex<i32>,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                let self_value = self.value.lock().unwrap();
                let other_value = other.value.lock().unwrap();
                *self_value == *other_value
            }
        }

        roundtrip_with(
            &Test {
                value: Mutex::new(10),
            },
            |a, b| {
                let a_value = a.value.lock().unwrap();
                assert_eq!(b.value, *a_value);
            },
        );
    }

    #[test]
    fn with_hash_map_mapkv() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[rkyv(with = MapKV<InlineAsBox, InlineAsBox>)]
            inner: HashMap<&'a str, &'a str>,
        }

        let mut inner = HashMap::new();
        inner.insert("cat", "hat");

        let value = Test { inner };

        to_archived(&value, |archived| {
            assert_eq!(&**archived.inner.get("cat").unwrap(), "hat");
        });
    }

    #[test]
    fn with_btree_map_mapkv() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[rkyv(with = MapKV<InlineAsBox, InlineAsBox>)]
            inner: BTreeMap<&'a str, &'a str>,
        }

        let mut inner = BTreeMap::new();
        inner.insert("cat", "hat");

        let value = Test { inner };

        to_archived(&value, |archived| {
            assert_eq!(&**archived.inner.get("cat").unwrap(), "hat");
        });
    }

    #[test]
    fn roundtrip_rwlock() {
        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[rkyv(crate, derive(Debug, PartialEq))]
        struct Test {
            #[rkyv(with = Lock)]
            value: RwLock<i32>,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                let self_value = self.value.try_read().unwrap();
                let other_value = other.value.try_read().unwrap();
                *self_value == *other_value
            }
        }

        roundtrip_with(
            &Test {
                value: RwLock::new(10),
            },
            |a, b| {
                let a_value = a.value.try_read().unwrap();
                assert_eq!(b.value, *a_value);
            },
        );
    }

    #[test]
    fn roundtrip_os_string() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug, PartialEq))]
        struct Test {
            #[rkyv(with = AsString)]
            value: OsString,
        }

        roundtrip_with(
            &Test {
                value: OsString::from("hello world"),
            },
            |a, b| {
                assert_eq!(a.value.as_os_str().to_str().unwrap(), b.value);
            },
        );
    }

    #[test]
    fn roundtrip_path_buf() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug, PartialEq))]
        struct Test {
            #[rkyv(with = AsString)]
            value: PathBuf,
        }

        roundtrip_with(
            &Test {
                value: PathBuf::from("hello world"),
            },
            |a, b| {
                assert_eq!(a.value.as_os_str().to_str().unwrap(), b.value);
            },
        );
    }
}
