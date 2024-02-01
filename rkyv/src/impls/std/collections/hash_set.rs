use crate::{
    collections::swiss_set::{ArchivedSwissSet, SwissSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Serialize,
};
use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};
use rancor::{Error, Fallible};
use std::collections::HashSet;

impl<K: Archive + Hash + Eq, S> Archive for HashSet<K, S>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedSwissSet<K::Archived>;
    type Resolver = SwissSetResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedSwissSet::<K::Archived>::resolve_from_len(
            self.len(),
            (7, 8),
            pos,
            resolver,
            out,
        );
    }
}

impl<K, S, RS> Serialize<S> for HashSet<K, RS>
where
    K::Archived: Hash + Eq,
    K: Serialize<S> + Hash + Eq,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Error,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedSwissSet::serialize_from_iter(self.iter(), (7, 8), serializer)
    }
}

impl<K, D, S> Deserialize<HashSet<K, S>, D> for ArchivedSwissSet<K::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D> + Hash + Eq,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<HashSet<K, S>, D::Error> {
        let mut result = HashSet::with_hasher(S::default());
        for k in self.iter() {
            result.insert(k.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq, S: BuildHasher>
    PartialEq<HashSet<K, S>> for ArchivedSwissSet<AK>
{
    #[inline]
    fn eq(&self, other: &HashSet<K, S>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|key| other.get(key).is_some())
        }
    }
}

impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq, S: BuildHasher>
    PartialEq<ArchivedSwissSet<AK>> for HashSet<K, S>
{
    #[inline]
    fn eq(&self, other: &ArchivedSwissSet<AK>) -> bool {
        other.eq(self)
    }
}
