use core::ops::ControlFlow;

use rancor::{Fallible, Source};

use crate::{
    alloc::collections::BTreeSet,
    collections::btree_set::{ArchivedBTreeSet, BTreeSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive + Ord> Archive for BTreeSet<K>
where
    K::Archived: Ord,
{
    type Archived = ArchivedBTreeSet<K::Archived>;
    type Resolver = BTreeSetResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedBTreeSet::<K::Archived>::resolve_from_len(
            self.len(),
            resolver,
            out,
        );
    }
}

impl<K, S> Serialize<S> for BTreeSet<K>
where
    K: Serialize<S> + Ord,
    K::Archived: Ord,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Self::Archived::serialize_from_ordered_iter::<_, K, _>(
            self.iter(),
            serializer,
        )
    }
}

impl<K, D> Deserialize<BTreeSet<K>, D> for ArchivedBTreeSet<K::Archived>
where
    K: Archive + Ord,
    K::Archived: Deserialize<K, D> + Ord,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BTreeSet<K>, D::Error> {
        let mut result = BTreeSet::new();
        let r = self.visit(|ak| {
            let k = match ak.deserialize(deserializer) {
                Ok(k) => k,
                Err(e) => return ControlFlow::Break(e),
            };
            result.insert(k);
            ControlFlow::Continue(())
        });
        match r {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

impl<K, AK: PartialEq<K>> PartialEq<BTreeSet<K>> for ArchivedBTreeSet<AK> {
    fn eq(&self, other: &BTreeSet<K>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            let mut iter = other.iter();
            self.visit(|ak| {
                if let Some(k) = iter.next() {
                    if ak.eq(k) {
                        return ControlFlow::Continue(());
                    }
                }
                ControlFlow::Break(())
            })
            .is_none()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        alloc::{collections::BTreeSet, string::ToString},
        api::test::{roundtrip, to_archived},
    };

    #[test]
    fn roundtrip_btree_set() {
        let mut value = BTreeSet::new();
        value.insert("foo".to_string());
        value.insert("bar".to_string());
        value.insert("baz".to_string());
        value.insert("bat".to_string());

        roundtrip(&value);
    }

    #[test]
    fn roundtrip_btree_set_zst() {
        let mut value = BTreeSet::new();
        value.insert(());

        roundtrip(&value);
    }

    #[test]
    fn btree_map_iter() {
        let mut value = BTreeSet::<i32>::new();
        value.insert(10);
        value.insert(20);
        value.insert(40);
        value.insert(80);

        to_archived(&value, |archived| {
            let mut i = archived.iter().map(|v| v.to_native());
            assert_eq!(i.next(), Some(10));
            assert_eq!(i.next(), Some(20));
            assert_eq!(i.next(), Some(40));
            assert_eq!(i.next(), Some(80));
            assert_eq!(i.next(), None);
        });
    }
}
