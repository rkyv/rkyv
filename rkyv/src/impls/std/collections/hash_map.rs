use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};
use std::collections::HashMap;

use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::map::{ArchivedHashMap, HashMapResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K, V: Archive, S> Archive for HashMap<K, V, S>
where
    K: Archive + Hash + Eq,
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashMap<K::Archived, V::Archived>;
    type Resolver = HashMapResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedHashMap::resolve_from_len(self.len(), (7, 8), resolver, out);
    }
}

impl<K, V, S, RandomState> Serialize<S> for HashMap<K, V, RandomState>
where
    K: Serialize<S> + Hash + Eq,
    K::Archived: Hash + Eq,
    V: Serialize<S>,
    S: Fallible + Writer + Allocator + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedHashMap::<K::Archived, V::Archived>::serialize_from_iter(
            self.iter(),
            (7, 8),
            serializer,
        )
    }
}

impl<K, V, D, S> Deserialize<HashMap<K, V, S>, D>
    for ArchivedHashMap<K::Archived, V::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D> + Hash + Eq,
    V: Archive,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<HashMap<K, V, S>, D::Error> {
        let mut result =
            HashMap::with_capacity_and_hasher(self.len(), S::default());
        for (k, v) in self.iter() {
            result.insert(
                k.deserialize(deserializer)?,
                v.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<K, V, AK, AV, S> PartialEq<HashMap<K, V, S>> for ArchivedHashMap<AK, AV>
where
    K: Hash + Eq + Borrow<AK>,
    AK: Hash + Eq,
    AV: PartialEq<V>,
    S: BuildHasher,
{
    fn eq(&self, other: &HashMap<K, V, S>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|(key, value)| {
                other.get(key).map_or(false, |v| value.eq(v))
            })
        }
    }
}

impl<K, V, AK, AV> PartialEq<ArchivedHashMap<AK, AV>> for HashMap<K, V>
where
    K: Hash + Eq + Borrow<AK>,
    AK: Hash + Eq,
    AV: PartialEq<V>,
{
    fn eq(&self, other: &ArchivedHashMap<AK, AV>) -> bool {
        other.eq(self)
    }
}

#[cfg(test)]
mod tests {
    use core::hash::BuildHasher;
    use std::collections::HashMap;

    use ahash::RandomState;
    use rancor::Panic;

    use crate::{
        access_unchecked,
        test::{roundtrip, roundtrip_with, to_bytes},
        Archive, Archived, Deserialize, Serialize,
    };

    fn assert_equal<S: BuildHasher>(
        a: &HashMap<String, String, S>,
        b: &Archived<HashMap<String, String, S>>,
    ) {
        assert_eq!(a.len(), b.len());

        for (key, value) in a.iter() {
            assert!(b.contains_key(key.as_str()));
            assert_eq!(&b[key.as_str()], value);
        }

        for (key, value) in b.iter() {
            assert!(a.contains_key(key.as_str()));
            assert_eq!(&a[key.as_str()], value);
        }
    }

    #[test]
    fn roundtrip_hash_map() {
        let mut hash_map = HashMap::new();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        roundtrip_with(&hash_map, assert_equal);
    }

    #[test]
    fn roundtrip_hash_map_zsts() {
        let mut value = HashMap::new();
        value.insert((), 10);
        roundtrip(&value);

        let mut value = HashMap::new();
        value.insert((), ());
        roundtrip(&value);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[allow(deprecated)]
    fn roundtrip_hash_map_with_custom_hasher() {
        use std::collections::HashMap;

        roundtrip(&HashMap::<i8, i32, RandomState>::default());

        let mut hash_map: HashMap<i8, _, RandomState> = HashMap::default();
        hash_map.insert(1, 2);
        hash_map.insert(3, 4);
        hash_map.insert(5, 6);
        hash_map.insert(7, 8);

        roundtrip(&hash_map);

        let mut hash_map: HashMap<_, _, RandomState> = HashMap::default();
        hash_map.insert("hello".to_string(), "world".to_string());
        hash_map.insert("foo".to_string(), "bar".to_string());
        hash_map.insert("baz".to_string(), "bat".to_string());

        roundtrip_with(&hash_map, assert_equal);
    }

    #[test]
    fn get_with() {
        #[derive(Archive, Serialize, Deserialize, Eq, Hash, PartialEq)]
        #[archive(crate)]
        #[archive_attr(derive(Eq, Hash, PartialEq))]
        pub struct Pair(String, String);

        let mut hash_map = HashMap::new();
        hash_map.insert(
            Pair("my".to_string(), "key".to_string()),
            "value".to_string(),
        );
        hash_map.insert(
            Pair("wrong".to_string(), "key".to_string()),
            "wrong value".to_string(),
        );

        to_bytes::<_, Panic>(&hash_map, |bytes| {
            let archived_value = unsafe {
                access_unchecked::<Archived<HashMap<Pair, String>>>(&bytes)
            };

            let get_with = archived_value
                .get_with(&("my", "key"), |input_key, key| {
                    &(key.0.as_str(), key.1.as_str()) == input_key
                })
                .unwrap();

            assert_eq!(get_with.as_str(), "value");
        });
    }
}
