use crate::{
    collections::index_map::{ArchivedIndexMap, IndexMapResolver},
    ser::{ScratchSpace, Serializer},
    Archive, Deserialize, Fallible, Serialize,
};
use core::hash::{BuildHasher, Hash};
use indexmap::IndexMap;

impl<K: Archive, V: Archive, S> Archive for IndexMap<K, V, S> {
    type Archived = ArchivedIndexMap<K::Archived, V::Archived>;
    type Resolver = IndexMapResolver;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedIndexMap::resolve_from_len(self.len(), pos, resolver, out);
    }
}

impl<K, V, S, RandomState> Serialize<S> for IndexMap<K, V, RandomState>
where
    K: Hash + Eq + Serialize<S>,
    V: Serialize<S>,
    S: Fallible + ScratchSpace + Serializer + ?Sized,
    RandomState: BuildHasher,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<IndexMapResolver, S::Error> {
        unsafe {
            ArchivedIndexMap::serialize_from_iter_index(
                self.iter(),
                |k| self.get_index_of(k).unwrap(),
                serializer,
            )
        }
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
    use crate::{access_unchecked, deserialize};
    use indexmap::{indexmap, IndexMap};
    use rancor::{Failure, Infallible};

    #[test]
    fn index_map() {
        let value = indexmap! {
            String::from("foo") => 10,
            String::from("bar") => 20,
            String::from("baz") => 40,
            String::from("bat") => 80,
        };

        let result = crate::to_bytes::<_, 4096, Failure>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<IndexMap<String, i32>>(result.as_ref())
        };

        assert_eq!(value.len(), archived.len());
        for (k, v) in value.iter() {
            let (ak, av) = archived.get_key_value(k.as_str()).unwrap();
            assert_eq!(k, ak);
            assert_eq!(v, av);
        }

        let deserialized = deserialize::<IndexMap<String, i32>, _, Infallible>(
            archived,
            &mut (),
        )
        .unwrap();
        assert_eq!(value, deserialized);
    }

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validate_index_map() {
        use rancor::Failure;

        // use crate::check_archived_root;

        let value = indexmap! {
            String::from("foo") => 10,
            String::from("bar") => 20,
            String::from("baz") => 40,
            String::from("bat") => 80,
        };

        let _result = crate::to_bytes::<_, 4096, Failure>(&value).unwrap();
        // TODO: re-enable after implementing CheckBytes for hash maps
        // check_archived_root::<IndexMap<String, i32>, Failure>(result.as_ref())
        //     .expect("failed to validate archived index map");
    }
}
