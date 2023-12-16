use crate::{
    collections::btree_set::{ArchivedBTreeSet, BTreeSetResolver},
    ser::Serializer,
    Archive, Deserialize, Serialize,
};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeSet;
use rancor::Fallible;
#[cfg(feature = "std")]
use std::collections::BTreeSet;

impl<K: Archive + Ord> Archive for BTreeSet<K>
where
    K::Archived: Ord,
{
    type Archived = ArchivedBTreeSet<K::Archived>;
    type Resolver = BTreeSetResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedBTreeSet::<K::Archived>::resolve_from_len(
            self.len(),
            pos,
            resolver,
            out,
        );
    }
}

impl<K, S> Serialize<S> for BTreeSet<K>
where
    K: Serialize<S> + Ord,
    K::Archived: Ord,
    S: Fallible + Serializer + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        unsafe {
            ArchivedBTreeSet::serialize_from_reverse_iter(
                self.iter().rev(),
                serializer,
            )
        }
    }
}

impl<K, D> Deserialize<BTreeSet<K>, D> for ArchivedBTreeSet<K::Archived>
where
    K: Archive + Ord,
    K::Archived: Deserialize<K, D> + Ord,
    D: Fallible + ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BTreeSet<K>, D::Error> {
        let mut result = BTreeSet::new();
        for k in self.iter() {
            result.insert(k.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<K, AK: PartialEq<K>> PartialEq<BTreeSet<K>> for ArchivedBTreeSet<AK> {
    #[inline]
    fn eq(&self, other: &BTreeSet<K>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().zip(other.iter()).all(|(a, b)| a.eq(b))
        }
    }
}

impl<K, AK: PartialEq<K>> PartialEq<ArchivedBTreeSet<AK>> for BTreeSet<K> {
    #[inline]
    fn eq(&self, other: &ArchivedBTreeSet<AK>) -> bool {
        other.eq(self)
    }
}
