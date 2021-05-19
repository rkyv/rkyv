use crate::{
    with::{ArchiveWith, DeserializeWith, Immutable, SerializeWith},
    Archive, Deserialize, Fallible, Serialize,
};
use core::mem::MaybeUninit;
use std::sync::{Mutex, RwLock};

/// A wrapper that locks a mutex or lock and serializes the value immutably.
pub struct Lock;

impl<F: Archive> ArchiveWith<Mutex<F>> for Lock {
    type Archived = Immutable<F::Archived>;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &Mutex<F>,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        field
            .lock()
            .unwrap()
            .resolve(pos, resolver, Immutable::as_inner(out));
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<Mutex<F>, S> for Lock {
    fn serialize_with(field: &Mutex<F>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.lock().unwrap().serialize(serializer)
    }
}

impl<F: Deserialize<T, D>, T, D: Fallible + ?Sized> DeserializeWith<Immutable<F>, Mutex<T>, D>
    for Lock
{
    fn deserialize_with(field: &Immutable<F>, deserializer: &mut D) -> Result<Mutex<T>, D::Error> {
        Ok(Mutex::new(field.value().deserialize(deserializer)?))
    }
}

impl<F: Archive> ArchiveWith<RwLock<F>> for Lock {
    type Archived = Immutable<F::Archived>;
    type Resolver = F::Resolver;

    fn resolve_with(
        field: &RwLock<F>,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        field
            .read()
            .unwrap()
            .resolve(pos, resolver, Immutable::as_inner(out));
    }
}

impl<F: Serialize<S>, S: Fallible + ?Sized> SerializeWith<RwLock<F>, S> for Lock {
    fn serialize_with(field: &RwLock<F>, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        field.read().unwrap().serialize(serializer)
    }
}

impl<F: Deserialize<T, D>, T, D: Fallible + ?Sized> DeserializeWith<Immutable<F>, RwLock<T>, D>
    for Lock
{
    fn deserialize_with(field: &Immutable<F>, deserializer: &mut D) -> Result<RwLock<T>, D::Error> {
        Ok(RwLock::new(field.value().deserialize(deserializer)?))
    }
}
