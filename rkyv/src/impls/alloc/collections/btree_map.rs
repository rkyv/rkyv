use core::ops::ControlFlow;

use rancor::{Fallible, Source};

use crate::{
    alloc::collections::BTreeMap,
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
        Self::Archived::serialize_from_ordered_iter::<_, _, _, K, V, _>(
            self.iter(),
            serializer,
        )
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
    use core::ops::ControlFlow;

    use crate::{
        alloc::{
            collections::BTreeMap,
            string::{String, ToString},
            vec,
            vec::Vec,
        },
        api::test::{roundtrip, to_archived},
        collections::btree_map::ArchivedBTreeMap,
        primitive::ArchivedI32,
        seal::Seal,
        Archive, Deserialize, Serialize,
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
    fn roundtrip_btree_map_increasing_sizes() {
        // These sizes are chosen based on a branching factor of 6.
        // 0-5: Leaf root node, variably-filled
        // 6: Inner root node and one leaf node with one element
        // 17: Inner root node, two filled leaf nodes, and one with two elements
        // 35: Two full levels
        // 36: Two full levels and one additional element
        // 112: Two full levels and a third level half-filled
        // 215: Three full levels
        const SIZES: &[usize] = &[0, 1, 2, 3, 4, 5, 6, 17, 35, 36, 112, 215];
        for &size in SIZES {
            let mut value = BTreeMap::new();
            for i in 0..size {
                value.insert(i.to_string(), i as i32);
            }

            roundtrip(&value);
        }
    }

    // This test creates structures too big to fit in 16-bit offsets, and MIRI
    // can't run it quickly enough.
    #[cfg(not(any(feature = "pointer_width_16", miri)))]
    #[test]
    fn roundtrip_large_btree_map() {
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
        #[rkyv(crate, compare(PartialEq), derive(Debug))]
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
    fn mutable_btree_map() {
        let mut value = BTreeMap::new();
        value.insert("foo".to_string(), 10);
        value.insert("bar".to_string(), 20);
        value.insert("baz".to_string(), 40);
        value.insert("bat".to_string(), 80);

        to_archived(&value, |mut archived| {
            ArchivedBTreeMap::visit_seal(
                archived.as_mut(),
                |_, mut v: Seal<'_, ArchivedI32>| {
                    *v = ArchivedI32::from_native(v.to_native() + 10);
                    ControlFlow::<(), ()>::Continue(())
                },
            );
            assert_eq!(archived.get("foo").map(|x| x.to_native()), Some(20));
            assert_eq!(archived.get("bar").map(|x| x.to_native()), Some(30));
            assert_eq!(archived.get("baz").map(|x| x.to_native()), Some(50));
            assert_eq!(archived.get("bat").map(|x| x.to_native()), Some(90));

            *ArchivedBTreeMap::get_seal(archived.as_mut(), "foo").unwrap() =
                ArchivedI32::from_native(123);
            *ArchivedBTreeMap::get_seal(archived.as_mut(), "bat").unwrap() =
                ArchivedI32::from_native(456);

            assert_eq!(archived.get("foo").map(|x| x.to_native()), Some(123));
            assert_eq!(archived.get("bat").map(|x| x.to_native()), Some(456));
        });
    }

    #[test]
    fn btree_map_iter() {
        let mut value = BTreeMap::<String, i32>::new();
        value.insert("foo".to_string(), 10);
        value.insert("bar".to_string(), 20);
        value.insert("baz".to_string(), 40);
        value.insert("bat".to_string(), 80);

        to_archived(&value, |archived| {
            let mut i =
                archived.iter().map(|(k, v)| (k.as_str(), v.to_native()));
            assert_eq!(i.next(), Some(("bar", 20)));
            assert_eq!(i.next(), Some(("bat", 80)));
            assert_eq!(i.next(), Some(("baz", 40)));
            assert_eq!(i.next(), Some(("foo", 10)));
            assert_eq!(i.next(), None);
        });
    }

    #[test]
    fn btree_map_mutable_iter() {
        let mut value = BTreeMap::<String, i32>::new();
        value.insert("foo".to_string(), 10);
        value.insert("bar".to_string(), 20);
        value.insert("baz".to_string(), 40);
        value.insert("bat".to_string(), 80);

        to_archived(&value, |archived| {
            let mut i =
                archived.iter().map(|(k, v)| (k.as_str(), v.to_native()));
            assert_eq!(i.next(), Some(("bar", 20)));
            assert_eq!(i.next(), Some(("bat", 80)));
            assert_eq!(i.next(), Some(("baz", 40)));
            assert_eq!(i.next(), Some(("foo", 10)));
            assert_eq!(i.next(), None);
        });
    }

    // MIRI can't run this test quickly enough
    #[cfg(not(miri))]
    #[test]
    fn size_matches_iter_len() {
        #[derive(Archive, Deserialize, Serialize)]
        #[rkyv(crate)]
        struct Container {
            transforms: BTreeMap<String, String>,
        }

        impl Container {
            pub fn fill(count: usize) -> Self {
                let mut transforms = BTreeMap::new();
                for i in 0..count {
                    transforms.insert(i.to_string(), i.to_string());
                }
                Container { transforms }
            }

            pub fn check(&self, expected: usize) {
                to_archived(self, |archived| {
                    assert_eq!(archived.transforms.len(), expected);
                    let mut count = 0;
                    for (..) in archived.transforms.iter() {
                        count += 1;
                    }
                    assert_eq!(count, expected);
                });
            }
        }

        for expected in 0..=200 {
            let container = Container::fill(expected);
            container.check(expected);
        }
    }
}
