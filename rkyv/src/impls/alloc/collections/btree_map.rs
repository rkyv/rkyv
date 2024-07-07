#[cfg(not(feature = "std"))]
use alloc::collections::BTreeMap;
use core::ops::ControlFlow;
#[cfg(feature = "std")]
use std::collections::BTreeMap;

use rancor::{Fallible, Source};

use crate::{
    collections::btree_map::{ArchivedBTreeMap, BTreeMapResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive + Ord, V: Archive> Archive for BTreeMap<K, V>
where
    K::Archived: Ord,
{
    type Archived = ArchivedBTreeMap<K::Archived, V::Archived>;
    type Resolver = BTreeMapResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        Self::Archived::resolve_from_len(self.len(), resolver, out);
    }
}

impl<K, V, S> Serialize<S> for BTreeMap<K, V>
where
    K: Serialize<S> + Ord,
    K::Archived: Ord,
    V: Serialize<S>,
    S: Allocator + Fallible + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Self::Archived::serialize_from_ordered_iter(self.iter(), serializer)
    }
}

impl<K, V, D> Deserialize<BTreeMap<K, V>, D>
    for ArchivedBTreeMap<K::Archived, V::Archived>
where
    K: Archive + Ord,
    K::Archived: Deserialize<K, D> + Ord,
    V: Archive,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<BTreeMap<K, V>, D::Error> {
        let mut result = BTreeMap::new();
        let r = self.visit(|ak, av| {
            let k = match ak.deserialize(deserializer) {
                Ok(k) => k,
                Err(e) => return ControlFlow::Break(e),
            };
            let v = match av.deserialize(deserializer) {
                Ok(v) => v,
                Err(e) => return ControlFlow::Break(e),
            };
            result.insert(k, v);
            ControlFlow::Continue(())
        });
        match r {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

impl<K, V, AK, AV> PartialEq<BTreeMap<K, V>> for ArchivedBTreeMap<AK, AV>
where
    AK: PartialEq<K>,
    AV: PartialEq<V>,
{
    fn eq(&self, other: &BTreeMap<K, V>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            let mut iter = other.iter();
            self.visit(|ak, av| {
                if let Some((k, v)) = iter.next() {
                    if ak.eq(k) && av.eq(v) {
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
    use rancor::Failure;

    use super::BTreeMap;
    use crate::{
        access, test::roundtrip, util::Align, Archive, Archived, Deserialize,
        Serialize,
    };

    #[test]
    fn roundtrip_btree_map() {
        let mut value = BTreeMap::new();
        value.insert("foo".to_string(), 10);
        value.insert("bar".to_string(), 20);
        value.insert("baz".to_string(), 40);
        value.insert("bat".to_string(), 80);

        roundtrip(&value);
    }

    #[test]
    fn roundtrip_empty_btree_map() {
        roundtrip(&BTreeMap::<String, i32>::new());
    }

    #[test]
    fn roundtrip_btree_map_zst() {
        let mut value = BTreeMap::new();
        value.insert(0, ());
        value.insert(1, ());
        roundtrip(&value);

        let mut value = BTreeMap::new();
        value.insert((), 10);
        roundtrip(&value);

        let mut value = BTreeMap::new();
        value.insert((), ());
        roundtrip(&value);
    }

    #[test]
    fn roundtrip_large_btree_map() {
        // This test creates structures too big to fit in 16-bit offsets, and
        // MIRI can't run it quickly enough.
        #[cfg(any(feature = "pointer_width_16", miri))]
        const ENTRIES: usize = 100;
        #[cfg(not(miri))]
        const ENTRIES: usize = 100_000;

        let mut value = BTreeMap::new();
        for i in 0..ENTRIES {
            value.insert(i.to_string(), i as i32);
        }

        roundtrip(&value);
    }

    #[test]
    fn roundtrip_btree_map_with_struct_member() {
        #[derive(
            Archive, Serialize, Deserialize, Debug, Default, PartialEq,
        )]
        #[rkyv(crate, check_bytes, compare(PartialEq))]
        #[rkyv_attr(derive(Debug))]
        pub struct MyType {
            pub some_list: BTreeMap<String, Vec<f32>>,
            pub values: Vec<f32>,
        }

        let mut value = MyType::default();

        value
            .some_list
            .entry("Asdf".to_string())
            .and_modify(|e| e.push(1.0))
            .or_insert_with(|| vec![2.0]);

        roundtrip(&value);
    }

    #[test]
    fn check_invalid_btreemap() {
        let data = Align([
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0x30, 0, 0x00, 0x00, 0x00, 0x0c, 0xa5,
            0xf0, 0xff, 0xff, 0xff,
        ]);
        access::<Archived<BTreeMap<u8, Box<u8>>>, Failure>(&*data).unwrap_err();
    }
}
