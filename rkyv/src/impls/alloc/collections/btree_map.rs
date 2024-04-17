#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use rancor::Fallible;

use crate::{
    collections::btree_map::{ArchivedBTreeMap, BTreeMapResolver},
    ser::Writer,
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive + Ord, V: Archive> Archive for BTreeMap<K, V>
where
    K::Archived: Ord,
{
    type Archived = ArchivedBTreeMap<K::Archived, V::Archived>;
    type Resolver = BTreeMapResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        ArchivedBTreeMap::resolve_from_len(self.len(), resolver, out);
    }
}

impl<K, V, S> Serialize<S> for BTreeMap<K, V>
where
    K: Serialize<S> + Ord,
    K::Archived: Ord,
    V: Serialize<S>,
    S: Fallible + Writer + ?Sized,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        unsafe {
            ArchivedBTreeMap::serialize_from_reverse_iter(
                self.iter().rev(),
                serializer,
            )
        }
    }
}

impl<K: Archive + Ord, V: Archive, D: Fallible + ?Sized>
    Deserialize<BTreeMap<K, V>, D>
    for ArchivedBTreeMap<K::Archived, V::Archived>
where
    K::Archived: Deserialize<K, D> + Ord,
    V::Archived: Deserialize<V, D>,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BTreeMap<K, V>, D::Error> {
        let mut result = BTreeMap::new();
        for (key, value) in self.iter() {
            result.insert(
                key.deserialize(deserializer)?,
                value.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<K, V, AK: PartialEq<K>, AV: PartialEq<V>> PartialEq<BTreeMap<K, V>>
    for ArchivedBTreeMap<AK, AV>
{
    #[inline]
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter()
                .zip(other.iter())
                .all(|(a, b)| a.0.eq(b.0) && a.1.eq(b.1))
        }
    }
}

impl<K, V, AK: PartialEq<K>, AV: PartialEq<V>>
    PartialEq<ArchivedBTreeMap<AK, AV>> for BTreeMap<K, V>
{
    #[inline]
    fn eq(&self, other: &ArchivedBTreeMap<AK, AV>) -> bool {
        other.eq(self)
    }
}
