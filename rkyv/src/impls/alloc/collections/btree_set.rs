use crate::{
    collections::btree_set::{ArchivedBTreeSet, BTreeSetResolver},
    ser::Serializer,
    Archive, Deserialize, Serialize,
};
#[cfg(not(feature = "std"))]
use alloc::collections::BTreeSet;
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

impl<K: Serialize<S, E> + Ord, S: Serializer<E> + ?Sized, E> Serialize<S, E> for BTreeSet<K>
where
    K::Archived: Ord,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        unsafe {
            ArchivedBTreeSet::serialize_from_reverse_iter(
                self.iter().rev(),
                serializer,
            )
        }
    }
}

impl<K, D, E> Deserialize<BTreeSet<K>, D, E> for ArchivedBTreeSet<K::Archived>
where
    K: Archive + Ord,
    K::Archived: Deserialize<K, D, E> + Ord,
    D: ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BTreeSet<K>, E> {
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
