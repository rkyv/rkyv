use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};

use hashbrown::HashMap;
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
        ArchivedHashMap::<K::Archived, V::Archived>::serialize_from_iter::<
            _,
            _,
            _,
            K,
            V,
            _,
        >(self.iter(), (7, 8), serializer)
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

#[cfg(test)]
mod tests {
    use core::hash::BuildHasherDefault;

    use super::HashMap;
    use crate::{
        alloc::string::String, api::test::roundtrip_with, hash::FxHasher64,
    };

    #[test]
    fn index_map() {
        let mut value =
            HashMap::with_hasher(BuildHasherDefault::<FxHasher64>::default());
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
}
