//! rkyv implementation for `IndexMap`.

use crate::{
    impls::indexmap::index_map::{ArchivedIndexMap, IndexMapResolver},
    ser::Serializer,
    Archive,
    Deserialize,
    Fallible,
    Serialize,
};
use core::{hash::Hash, mem::MaybeUninit};
use indexmap::IndexMap;

impl<K: Archive, V: Archive> Archive for IndexMap<K, V> {
    type Archived = ArchivedIndexMap<K::Archived, V::Archived>;
    type Resolver = IndexMapResolver;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        ArchivedIndexMap::resolve_from_len(self.len(), pos, resolver, out);
    }
}

impl<K: Hash + Eq + Serialize<S>, V: Serialize<S>, S: Serializer + ?Sized> Serialize<S> for IndexMap<K, V> {
    fn serialize(&self, serializer: &mut S) -> Result<IndexMapResolver, S::Error> {
        unsafe {
            ArchivedIndexMap::serialize_from_iter_index(
                self.iter(),
                |k| self.get_index_of(k).unwrap(),
                serializer,
            )
        }
    }
}

impl<K, V, D> Deserialize<IndexMap<K, V>, D> for ArchivedIndexMap<K::Archived, V::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D>,
    V: Archive,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<IndexMap<K, V>, D::Error> {
        let mut result = IndexMap::with_capacity(self.len());
        for (k, v) in self.iter() {
            result.insert(k.deserialize(deserializer)?, v.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        archived_root,
        ser::{serializers::AlignedSerializer, Serializer},
        util::AlignedVec,
        Deserialize,
        Infallible,
    };
    use indexmap::{indexmap, IndexMap};

    #[test]
    fn index_map() {
        let value = indexmap! {
            String::from("foo") => 10,
            String::from("bar") => 20,
            String::from("baz") => 40,
            String::from("bat") => 80,
        };

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<IndexMap<String, i32>>(result.as_ref()) };

        assert_eq!(value.len(), archived.len());
        for (k, v) in value.iter() {
            let (ak, av) = archived.get_key_value(k.as_str()).unwrap();
            assert_eq!(k, ak);
            assert_eq!(v, av);
        }

        let deserialized = Deserialize::<IndexMap<String, i32>, _>::deserialize(
            archived,
            &mut Infallible,
        ).unwrap();
        assert!(value == deserialized);
    }
}
