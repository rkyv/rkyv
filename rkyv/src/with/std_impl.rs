use crate::{Archive, Deserialize, Fallible, Serialize, SerializeUnsized, std_impl::{ArchivedString, StringResolver, chd::{ArchivedHashMap, ArchivedHashMapResolver}}, with::{ArchiveWith, DeserializeWith, Immutable, SerializeWith}};
use core::{fmt, mem::MaybeUninit, str::FromStr};
use std::{ffi::OsString, hash::Hash, path::PathBuf, sync::{Mutex, RwLock}};

/// A wrapper that locks a lock and serializes the value immutably.
///
/// This wrapper can panic under very specific circumstances when:
///
/// 1. `serialize_with` is called and succeeds in locking the value to serialize it.
/// 2. Another thread locks the value and panics, poisoning the lock
/// 3. `resolve_with` is called and gets a poisoned value.
///
/// Unfortunately, it's not possible to work around this issue. If your code absolutely must not
/// panic under any circumstances, it's recommended that you lock your values and then serialize
/// them while locked.
pub struct Lock;

/// Errors that can occur while serializing a [`Lock`] wrapper
#[derive(Debug)]
pub enum LockError {
    /// The mutex was poisoned
    Poisoned,
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lock poisoned")
    }
}

impl std::error::Error for LockError {}

impl<F: Archive> ArchiveWith<Mutex<F>> for Lock {
    type Archived = Immutable<F::Archived>;
    type Resolver = F::Resolver;

    #[inline]
    fn resolve_with(
        field: &Mutex<F>,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        // Unfortunately, we have to unwrap here because resolve must be infallible
        //
        // An alternative would be to only implement ArchiveWith for Arc<Mutex<F>>, copy an Arc into
        // the resolver, and hold the guard in there as well (as a reference to the internal mutex).
        // This unfortunately will cause a deadlock if two Arcs to the same Mutex are serialized
        // before the first is resolved. The compromise is, unfortunately, to just unwrap poison
        // errors here and document it.
        field.lock().unwrap().resolve(pos, resolver, Immutable::as_inner(out));
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<Mutex<F>, S> for Lock
where
    S::Error: From<LockError>,
{
    #[inline]
    fn serialize_with(field: &Mutex<F>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.lock().map_err(|_| LockError::Poisoned)?.serialize(serializer)
    }
}

impl<F: Deserialize<T, D>, T, D: Fallible + ?Sized> DeserializeWith<Immutable<F>, Mutex<T>, D>
    for Lock
{
    #[inline]
    fn deserialize_with(field: &Immutable<F>, deserializer: &mut D) -> Result<Mutex<T>, D::Error> {
        Ok(Mutex::new(field.value().deserialize(deserializer)?))
    }
}

impl<F: Archive> ArchiveWith<RwLock<F>> for Lock {
    type Archived = Immutable<F::Archived>;
    type Resolver = F::Resolver;

    #[inline]
    fn resolve_with(
        field: &RwLock<F>,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        // Unfortunately, we have to unwrap here because resolve must be infallible
        //
        // An alternative would be to only implement ArchiveWith for Arc<Mutex<F>>, copy a an Arc
        // into the resolver, and hold the guard in there as well (as a reference to the internal
        // mutex). This unfortunately will cause a deadlock if two Arcs to the same Mutex are
        // serialized before the first is resolved. The compromise is, unfortunately, to just
        // unwrap poison errors here and document it.
        field.read().unwrap().resolve(pos, resolver, Immutable::as_inner(out));
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<RwLock<F>, S> for Lock
where
    S::Error: From<LockError>,
{
    #[inline]
    fn serialize_with(field: &RwLock<F>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.read().map_err(|_| LockError::Poisoned)?.serialize(serializer)
    }
}

impl<F: Deserialize<T, D>, T, D: Fallible + ?Sized> DeserializeWith<Immutable<F>, RwLock<T>, D>
    for Lock
{
    #[inline]
    fn deserialize_with(field: &Immutable<F>, deserializer: &mut D) -> Result<RwLock<T>, D::Error> {
        Ok(RwLock::new(field.value().deserialize(deserializer)?))
    }
}

/// A wrapper that attempts to convert a path to and from UTF-8.
pub struct ToString;

/// Errors that can occur when serializing a [`ToString`] wrapper.
#[derive(Debug)]
pub enum ToStringError {
    /// The `OsString` or `PathBuf` was not valid UTF-8.
    InvalidUTF8,
}

impl fmt::Display for ToStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8")
    }
}

impl std::error::Error for ToStringError {}

impl ArchiveWith<OsString> for ToString {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve_with(field: &OsString, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        // It's safe to unwrap here because if the OsString wasn't valid UTF-8 it would have failed
        // to serialize
        ArchivedString::resolve_from_str(field.to_str().unwrap(), pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<OsString, S> for ToString
where
    S::Error: From<ToStringError>,
    str: SerializeUnsized<S>,
{
    #[inline]
    fn serialize_with(field: &OsString, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(
            field.to_str().ok_or(ToStringError::InvalidUTF8)?,
            serializer,
        )
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedString, OsString, D> for ToString {
    #[inline]
    fn deserialize_with(field: &ArchivedString, _: &mut D) -> Result<OsString, D::Error> {
        Ok(OsString::from_str(field.as_str()).unwrap())
    }
}

impl ArchiveWith<PathBuf> for ToString {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    fn resolve_with(field: &PathBuf, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        // It's safe to unwrap here because if the OsString wasn't valid UTF-8 it would have failed
        // to serialize
        ArchivedString::resolve_from_str(field.to_str().unwrap(), pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized> SerializeWith<PathBuf, S> for ToString
where
    S::Error: From<ToStringError>,
    str: SerializeUnsized<S>,
{
    #[inline]
    fn serialize_with(field: &PathBuf, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(
            field.to_str().ok_or(ToStringError::InvalidUTF8)?,
            serializer,
        )
    }
}

impl<D: Fallible + ?Sized> DeserializeWith<ArchivedString, PathBuf, D> for ToString {
    #[inline]
    fn deserialize_with(field: &ArchivedString, _: &mut D) -> Result<PathBuf, D::Error> {
        Ok(PathBuf::from_str(field.as_str()).unwrap())
    }
}

/// A wrapper that attempts to convert a vector to and from `ArchivedHashMap`
///
/// rkyv's `ArchivedHashMap` uses a fairly different implementation than `HashMap` in the standard
/// library. Therefore, constructing `HashMap` and converting it to `ArchivedHashMap` will create
/// unnecessary hashes that will never be used. By labeling a vector `AsHashMap`, you can use its
/// archived version just like `ArchivedHashMap` without having costy `HashMap` creations.
///
/// Example:
///
/// ```rust
/// # use rkyv::{AlignedVec, Deserialize, Infallible, archived_root, ser::{Serializer, serializers::AlignedSerializer}};
/// #[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize, Debug, PartialEq, Eq)]
/// struct StructWithHashMap {
///     #[with(rkyv::with::AsHashMap)]
///     pub hash_map: Vec<(u32, String)>,
/// }
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// let original = StructWithHashMap {
///     hash_map: vec![(1, String::from("a")), (2, String::from("b"))]
/// };
/// serializer.serialize_value(&original).unwrap();
/// let buffer = serializer.into_inner();
/// let output = unsafe {
///     archived_root::<StructWithHashMap>(&buffer)
/// };
/// assert_eq!(output.hash_map.get(&1).unwrap(), &"a");
/// let deserialized: StructWithHashMap = output.deserialize(&mut Infallible).unwrap();
/// assert_eq!(deserialized, original);
/// ```
pub struct AsHashMap;
impl<K: Archive, V: Archive> ArchiveWith<Vec<(K, V)>> for AsHashMap {
    type Archived = ArchivedHashMap<K::Archived, V::Archived>;
    type Resolver = ArchivedHashMapResolver;
    #[inline]
    fn resolve_with(field: &Vec<(K,V)>, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        resolver.resolve_from_len(pos, field.len(), out);
    }
}
impl<K: Archive + Serialize<S> + Hash + Eq, V: Archive + Serialize<S>, S: crate::ser::Serializer + Fallible + ?Sized> SerializeWith<Vec<(K, V)>, S> for AsHashMap {
    #[inline]
    fn serialize_with(field: &Vec<(K, V)>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedHashMap::serialize_from_iter(field.iter().map(|(x, y)| (x, y)), field.len(), serializer)
    }
}
impl<K: Archive, V: Archive, D: Fallible + ?Sized> DeserializeWith<ArchivedHashMap<K::Archived, V::Archived>, Vec<(K, V)>, D> for AsHashMap where K::Archived: Deserialize<K, D>, V::Archived: Deserialize<V, D> {
    #[inline]
    fn deserialize_with(field: &ArchivedHashMap<K::Archived, V::Archived>, deserializer: &mut D) -> Result<Vec<(K, V)>, D::Error> {
        field.iter().map(|(k, v)| Ok((k.deserialize(deserializer)?, v.deserialize(deserializer)?))).collect()
    }
}
