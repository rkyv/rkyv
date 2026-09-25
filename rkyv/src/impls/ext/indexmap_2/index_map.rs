use core::hash::{BuildHasher, Hash};

use indexmap_2::IndexMap;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::{ArchivedIndexMap, IndexMapResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive, V: Archive, S> Archive for IndexMap<K, V, S> {
    type Archived = ArchivedIndexMap<K::Archived, V::Archived>;
    type Resolver = IndexMapResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedIndexMap::resolve_from_len(self.len(), (7, 8), resolver, out);
    }
}

impl<K, V, S, RandomState> Serialize<S> for IndexMap<K, V, RandomState>
where
    K: Hash + Eq + Serialize<S>,
    V: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<IndexMapResolver, S::Error> {
        ArchivedIndexMap::<K::Archived, V::Archived>::serialize_from_iter::<
            _,
            _,
            _,
            K,
            V,
            _,
        >(self.iter(), (7, 8), serializer)
    }
}

impl<K, V, D, S> Deserialize<IndexMap<K, V, S>, D>
    for ArchivedIndexMap<K::Archived, V::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D>,
    V: Archive,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<IndexMap<K, V, S>, D::Error> {
        let mut result =
            IndexMap::with_capacity_and_hasher(self.len(), S::default());
        for (k, v) in self.iter() {
            result.insert(
                k.deserialize(deserializer)?,
                v.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<UK, K, UV, V, S> PartialEq<IndexMap<UK, UV, S>> for ArchivedIndexMap<K, V>
where
    K: PartialEq<UK>,
    V: PartialEq<UV>,
    S: BuildHasher,
{
    fn eq(&self, other: &IndexMap<UK, UV, S>) -> bool {
        self.iter()
            .zip(other.iter())
            .all(|((ak, av), (bk, bv))| ak == bk && av == bv)
    }
}

#[cfg(test)]
mod tests {
    use core::hash::BuildHasherDefault;

    use indexmap_2::IndexMap;

    use crate::{
        alloc::string::String, api::test::roundtrip_with, hash::FxHasher64,
    };

    #[test]
    fn index_map() {
        let mut value =
            IndexMap::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(String::from("foo"), 10);
        value.insert(String::from("bar"), 20);
        value.insert(String::from("baz"), 40);
        value.insert(String::from("bat"), 80);

        roundtrip_with(&value, |a, b| {
            assert_eq!(a.len(), b.len());
            for (k, v) in a.iter() {
                let (ak, av) = b.get_key_value(k.as_str()).unwrap();
                assert_eq!(k, ak);
                assert_eq!(v, av);
            }
        });
    }

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validated_archive_never_looks_up_out_of_bounds() {
        use rancor::Error;

        use crate::{
            access, collections::swiss_table::ArchivedIndexMap,
            primitive::ArchivedU32, to_bytes,
        };

        let mut value =
            IndexMap::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(1u32, 10u32);
        value.insert(2u32, 20u32);
        value.insert(3u32, 30u32);

        let bytes = to_bytes::<Error>(&value).unwrap();

        // The hash table stores indices into `entries`. Corrupt every byte and
        // confirm that any archive which still validates is safe to look up.
        // Before #685, corruption that produced an out-of-bounds index passed
        // validation and then panicked on lookup.
        for i in 0..bytes.len() {
            let mut corrupted = bytes.clone();
            corrupted[i] = 0xff;
            if let Ok(map) = access::<
                ArchivedIndexMap<ArchivedU32, ArchivedU32>,
                Error,
            >(&corrupted)
            {
                for k in map.keys() {
                    let _ = map.get(k);
                }
            }
        }
    }
}
