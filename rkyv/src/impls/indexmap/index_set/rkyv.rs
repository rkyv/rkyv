//! rkyv implementation for `IndexSet`.

use crate::{
    impls::indexmap::{
        index_map::ArchivedIndexMap,
        index_set::{ArchivedIndexSet, IndexSetResolver},
    },
    ser::Serializer,
    Archive,
    Deserialize,
    Fallible,
    Serialize,
};
use core::{hash::Hash, mem::MaybeUninit};
use indexmap::IndexSet;

impl<K: Archive + Hash + Eq> Archive for IndexSet<K> {
    type Archived = ArchivedIndexSet<K::Archived>;
    type Resolver = IndexSetResolver;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        let (fp, fo) = out_field!(out.inner);
        ArchivedIndexMap::resolve_from_len(self.len(), pos + fp, resolver.0, fo);
    }
}

impl<K: Hash + Eq + Serialize<S>, S: Serializer + ?Sized> Serialize<S> for IndexSet<K> {
    fn serialize(&self, serializer: &mut S) -> Result<IndexSetResolver, S::Error> {
        unsafe {
            ArchivedIndexSet::serialize_from_iter_index(
                self.iter(),
                |k| self.get_index_of(k).unwrap(),
                serializer,
            )
        }
    }
}

impl<K, D> Deserialize<IndexSet<K>, D> for ArchivedIndexSet<K::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<IndexSet<K>, D::Error> {
        let mut result = IndexSet::with_capacity(self.len());
        for k in self.iter() {
            result.insert(k.deserialize(deserializer)?);
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
    use indexmap::{indexset, IndexSet};

    #[test]
    fn index_set() {
        let value = indexset! {
            String::from("foo"),
            String::from("bar"),
            String::from("baz"),
            String::from("bat"),
        };

        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer.serialize_value(&value).unwrap();
        let result = serializer.into_inner();
        let archived = unsafe { archived_root::<IndexSet<String>>(result.as_ref()) };

        assert_eq!(value.len(), archived.len());
        for k in value.iter() {
            let ak = archived.get(k.as_str()).unwrap();
            assert_eq!(k, ak);
        }

        let deserialized = Deserialize::<IndexSet<String>, _>::deserialize(
            archived,
            &mut Infallible,
        ).unwrap();
        assert!(value == deserialized);
    }
}
