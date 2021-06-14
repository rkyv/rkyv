use crate::{
    collections::hash_set::{ArchivedHashSet, HashSetResolver},
    ser::Serializer,
    Archive,
    Deserialize,
    Fallible,
    Serialize,
};
use core::{
    borrow::Borrow,
    hash::Hash,
    mem::MaybeUninit,
};
use std::collections::HashSet;

impl<K: Archive + Hash + Eq> Archive for HashSet<K>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashSet<K::Archived>;
    type Resolver = HashSetResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedHashSet::<K::Archived>::resolve_from_len(self.len(), pos, resolver, out);
    }
}

impl<K: Serialize<S> + Hash + Eq, S: Serializer + ?Sized> Serialize<S> for HashSet<K>
where
    K::Archived: Hash + Eq,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        unsafe {
            ArchivedHashSet::serialize_from_iter(self.iter(), self.len(), serializer)
        }
    }
}

impl<K: Archive + Hash + Eq, D: Fallible + ?Sized> Deserialize<HashSet<K>, D>
    for ArchivedHashSet<K::Archived>
where
    K::Archived: Deserialize<K, D> + Hash + Eq,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<HashSet<K>, D::Error> {
        let mut result = HashSet::new();
        for k in self.iter() {
            result.insert(k.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq> PartialEq<HashSet<K>> for ArchivedHashSet<AK> {
    #[inline]
    fn eq(&self, other: &HashSet<K>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|key| other.get(key).is_some())
        }
    }
}

impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq> PartialEq<ArchivedHashSet<AK>> for HashSet<K> {
    #[inline]
    fn eq(&self, other: &ArchivedHashSet<AK>) -> bool {
        other.eq(self)
    }
}