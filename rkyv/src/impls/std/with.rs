use core::fmt;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    ffi::{CStr, OsString},
    hash::Hash,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Mutex, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use rancor::{Fallible, OptionExt, ResultExt, Source};

use crate::{
    collections::util::{Entry, EntryAdapter},
    ffi::{ArchivedCString, CStringResolver},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    time::ArchivedDuration,
    vec::{ArchivedVec, VecResolver},
    with::{
        ArchiveWith, AsOwned, AsString, AsUnixTime, AsVec, DeserializeWith,
        Lock, SerializeWith, Unsafe,
    },
    Archive, Deserialize, Place, Serialize, SerializeUnsized,
};

// AsString

#[derive(Debug)]
struct InvalidUtf8;

impl fmt::Display for InvalidUtf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8")
    }
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidUtf8 {}

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

#[cfg(feature = "std")]
impl std::error::Error for LockPoisoned {}

impl<F: Archive> ArchiveWith<Mutex<F>> for Lock<Unsafe> {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &Mutex<F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let out = unsafe { out.cast_unchecked() };
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

impl<F, S> SerializeWith<Mutex<F>, S> for Lock<Unsafe>
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

impl<F, T, D> DeserializeWith<F, Mutex<T>, D> for Lock<Unsafe>
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

impl<F: Archive> ArchiveWith<RwLock<F>> for Lock<Unsafe> {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &RwLock<F>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        let out = unsafe { out.cast_unchecked() };
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

impl<F, S> SerializeWith<RwLock<F>, S> for Lock<Unsafe>
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

impl<F, T, D> DeserializeWith<F, RwLock<T>, D> for Lock<Unsafe>
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

impl<K: Archive, V: Archive> ArchiveWith<HashMap<K, V>> for AsVec {
    type Archived = ArchivedVec<Entry<K::Archived, V::Archived>>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &HashMap<K, V>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<K, V, S> SerializeWith<HashMap<K, V>, S> for AsVec
where
    K: Serialize<S>,
    V: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &HashMap<K, V>,
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
        HashMap<K, V>,
        D,
    > for AsVec
where
    K: Archive + Hash + Eq,
    V: Archive,
    K::Archived: Deserialize<K, D>,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<Entry<K::Archived, V::Archived>>,
        deserializer: &mut D,
    ) -> Result<HashMap<K, V>, D::Error> {
        let mut result = HashMap::with_capacity(field.len());
        for entry in field.iter() {
            result.insert(
                entry.key.deserialize(deserializer)?,
                entry.value.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<T: Archive> ArchiveWith<HashSet<T>> for AsVec {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    fn resolve_with(
        field: &HashSet<T>,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedVec::resolve_from_len(field.len(), resolver, out);
    }
}

impl<T, S> SerializeWith<HashSet<T>, S> for AsVec
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &HashSet<T>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::<T::Archived>::serialize_from_iter::<T, _, _>(
            field.iter(),
            serializer,
        )
    }
}

impl<T, D> DeserializeWith<ArchivedVec<T::Archived>, HashSet<T>, D> for AsVec
where
    T: Archive + Hash + Eq,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<HashSet<T>, D::Error> {
        let mut result = HashSet::with_capacity(field.len());
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
        Ok(UNIX_EPOCH + (*field).into())
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
        ffi::OsString,
        path::PathBuf,
        sync::{Mutex, RwLock},
    };

    use crate::{
        test::roundtrip_with,
        with::{AsString, Lock, Unsafe},
        Archive, Deserialize, Serialize,
    };

    #[test]
    fn roundtrip_mutex() {
        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[archive(crate, check_bytes)]
        #[archive_attr(derive(Debug, PartialEq))]
        struct Test {
            #[with(Lock<Unsafe>)]
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
    fn roundtrip_rwlock() {
        #[derive(Archive, Serialize, Deserialize, Debug)]
        #[archive(crate, check_bytes)]
        #[archive_attr(derive(Debug, PartialEq))]
        struct Test {
            #[with(Lock<Unsafe>)]
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
        #[archive(crate, check_bytes)]
        #[archive_attr(derive(Debug, PartialEq))]
        struct Test {
            #[with(AsString)]
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
        #[archive(crate, check_bytes)]
        #[archive_attr(derive(Debug, PartialEq))]
        struct Test {
            #[with(AsString)]
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
