use crate::{
    boxed::{ArchivedBox, BoxResolver},
    string::{ArchivedString, StringResolver},
    with::{ArchiveWith, DeserializeWith, Immutable, SerializeWith},
    Archive,
    ArchiveUnsized,
    Deserialize,
    Fallible,
    Serialize,
    SerializeUnsized,
};
use core::{fmt, mem::MaybeUninit, str::FromStr};
use std::{ffi::OsString, path::PathBuf, sync::{Mutex, RwLock}};

/// A wrapper that serializes a reference inline.
pub struct Inline;

impl<F: Archive> ArchiveWith<&F> for Inline {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    #[inline]
    unsafe fn resolve_with(
        field: &&F,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        field.resolve(pos, resolver, out);
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<&F, S> for Inline {
    #[inline]
    fn serialize_with(field: &&F, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.serialize(serializer)
    }
}

/// A wrapper that serializes a reference as if it were boxed.
pub struct Boxed;

impl<F: ArchiveUnsized + ?Sized> ArchiveWith<&F> for Boxed {
    type Archived = ArchivedBox<F::Archived>;
    type Resolver = BoxResolver<F::MetadataResolver>;

    #[inline]
    unsafe fn resolve_with(
        field: &&F,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        ArchivedBox::resolve_from_ref(*field, pos, resolver, out);
    }
}

impl<F: SerializeUnsized<S> + ?Sized, S: Fallible + ?Sized> SerializeWith<&F, S> for Boxed {
    #[inline]
    fn serialize_with(field: &&F, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedBox::serialize_from_ref(*field, serializer)
    }
}

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
    unsafe fn resolve_with(
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
    unsafe fn resolve_with(
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
    unsafe fn resolve_with(field: &OsString, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
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
    unsafe fn resolve_with(field: &PathBuf, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
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
