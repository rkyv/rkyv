use crate::{
    collections::index_set::{ArchivedIndexSet, IndexSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Fallible, Serialize,
};
use core::hash::{BuildHasher, Hash};
use indexmap::IndexSet;

impl<K: Archive, S> Archive for IndexSet<K, S> {
    type Archived = ArchivedIndexSet<K::Archived>;
    type Resolver = IndexSetResolver;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedIndexSet::resolve_from_len(self.len(), pos, resolver, out);
    }
}

impl<K, S, RandomState> Serialize<S> for IndexSet<K, RandomState>
where
    K: Hash + Eq + Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
    RandomState: BuildHasher,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<IndexSetResolver, S::Error> {
        unsafe {
            ArchivedIndexSet::serialize_from_iter_index(
                self.iter(),
                |k| self.get_index_of(k).unwrap(),
                serializer,
            )
        }
    }
}

impl<K, D, S> Deserialize<IndexSet<K, S>, D> for ArchivedIndexSet<K::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D>,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<IndexSet<K, S>, D::Error> {
        let mut result =
            IndexSet::with_capacity_and_hasher(self.len(), S::default());
        for k in self.iter() {
            result.insert(k.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<UK, K: PartialEq<UK>, S: BuildHasher> PartialEq<IndexSet<UK, S>>
    for ArchivedIndexSet<K>
{
    fn eq(&self, other: &IndexSet<UK, S>) -> bool {
        self.iter().eq(other.iter())
    }
}

#[cfg(test)]
mod tests {
    use crate::{access_unchecked, deserialize};
    use indexmap::{indexset, IndexSet};
    use rancor::{Failure, Infallible};

    #[test]
    fn index_set() {
        let value = indexset! {
            String::from("foo"),
            String::from("bar"),
            String::from("baz"),
            String::from("bat"),
        };

        let result = crate::to_bytes::<_, 4096, Failure>(&value).unwrap();
        let archived =
            unsafe { access_unchecked::<IndexSet<String>>(result.as_ref()) };

        assert_eq!(value.len(), archived.len());
        for k in value.iter() {
            let ak = archived.get(k.as_str()).unwrap();
            assert_eq!(k, ak);
        }

        let deserialized =
            deserialize::<IndexSet<String>, _, Infallible>(archived, &mut ())
                .unwrap();
        assert_eq!(value, deserialized);
    }

    // TODO: uncomment
    // #[cfg(feature = "bytecheck")]
    // #[test]
    // fn validate_index_set() {
    //     use crate::check_archived_root;

    //     let value = indexset! {
    //         String::from("foo"),
    //         String::from("bar"),
    //         String::from("baz"),
    //         String::from("bat"),
    //     };

    //     let result = crate::to_bytes::<_, 4096, Failure>(&value).unwrap();
    //     check_archived_root::<IndexSet<String>>(result.as_ref())
    //         .expect("failed to validate archived index set");
    // }
}
