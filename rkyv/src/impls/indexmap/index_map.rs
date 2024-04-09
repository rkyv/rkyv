use core::hash::{BuildHasher, Hash};

use indexmap::IndexMap;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::{ArchivedIndexMap, IndexMapResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Serialize,
};

impl<K: Archive, V: Archive, S> Archive for IndexMap<K, V, S> {
    type Archived = ArchivedIndexMap<K::Archived, V::Archived>;
    type Resolver = IndexMapResolver;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedIndexMap::resolve_from_len(
            self.len(),
            (7, 8),
            pos,
            resolver,
            out,
        );
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
        ArchivedIndexMap::<K::Archived, V::Archived>::serialize_from_iter(
            self.iter(),
            (7, 8),
            serializer,
        )
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
    #[cfg(not(feature = "std"))]
    use alloc::string::String;
    use core::hash::BuildHasherDefault;

    use indexmap::IndexMap;
    use rancor::{Error, Infallible};

    use crate::{
        access_unchecked, collections::swiss_table::ArchivedIndexMap,
        deserialize, hash::FxHasher64, string::ArchivedString, Archived,
    };

    #[test]
    fn index_map() {
        let mut value =
            IndexMap::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(String::from("foo"), 10);
        value.insert(String::from("bar"), 20);
        value.insert(String::from("baz"), 40);
        value.insert(String::from("bat"), 80);

        let result = crate::to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<ArchivedIndexMap<ArchivedString, Archived<i32>>>(
                result.as_ref(),
            )
        };

        assert_eq!(value.len(), archived.len());
        for (k, v) in value.iter() {
            let (ak, av) = archived.get_key_value(k.as_str()).unwrap();
            assert_eq!(k, ak);
            assert_eq!(v, av);
        }

        let deserialized = deserialize::<
            IndexMap<String, i32, BuildHasherDefault<FxHasher64>>,
            _,
            Infallible,
        >(archived, &mut ())
        .unwrap();
        assert_eq!(value, deserialized);
    }

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validate_index_map() {
        use crate::access;

        let mut value =
            IndexMap::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(String::from("foo"), 10);
        value.insert(String::from("bar"), 20);
        value.insert(String::from("baz"), 40);
        value.insert(String::from("bat"), 80);

        let result = crate::to_bytes::<Error>(&value).unwrap();
        access::<ArchivedIndexMap<ArchivedString, Archived<i32>>, Error>(
            result.as_ref(),
        )
        .expect("failed to validate archived index map");
    }
}
