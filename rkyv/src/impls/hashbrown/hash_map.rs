use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};

use hashbrown::HashMap;
use rancor::{Error, Fallible};

use crate::{
    collections::swiss_table::map::{ArchivedHashMap, HashMapResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Serialize,
};

impl<K: Archive + Hash + Eq, V: Archive, S> Archive for HashMap<K, V, S>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashMap<K::Archived, V::Archived>;
    type Resolver = HashMapResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedHashMap::resolve_from_len(
            self.len(),
            (7, 8),
            pos,
            resolver,
            out,
        );
    }
}

impl<K, V, S, RandomState> Serialize<S> for HashMap<K, V, RandomState>
where
    K: Serialize<S> + Hash + Eq,
    K::Archived: Hash + Eq,
    V: Serialize<S>,
    S: Fallible + Writer + Allocator + ?Sized,
    S::Error: Error,
{
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn eq(&self, other: &ArchivedHashMap<AK, AV>) -> bool {
        other.eq(self)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    use alloc::string::String;

    use hashbrown::HashMap;
    use rancor::Failure;

    use crate::{access, access_unchecked, deserialize, to_bytes};

    #[test]
    fn index_map() {
        let mut value = HashMap::new();
        value.insert(String::from("foo"), 10);
        value.insert(String::from("bar"), 20);
        value.insert(String::from("baz"), 40);
        value.insert(String::from("bat"), 80);

        let result = to_bytes::<_, 256, Failure>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<HashMap<String, i32>>(result.as_ref())
        };

        assert_eq!(value.len(), archived.len());
        for (k, v) in value.iter() {
            let (ak, av) = archived.get_key_value(k.as_str()).unwrap();
            assert_eq!(k, ak);
            assert_eq!(v, av);
        }

        let deserialized =
            deserialize::<HashMap<String, i32>, _, Failure>(archived, &mut ())
                .unwrap();
        assert_eq!(value, deserialized);
    }

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validate_index_map() {
        let mut value = HashMap::new();
        value.insert(String::from("foo"), 10);
        value.insert(String::from("bar"), 20);
        value.insert(String::from("baz"), 40);
        value.insert(String::from("bat"), 80);

        let bytes = to_bytes::<_, 256, Failure>(&value).unwrap();
        access::<HashMap<String, i32>, crate::rancor::Panic>(bytes.as_ref())
            .expect("failed to validate archived index map");
    }
}
