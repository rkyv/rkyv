use core::hash::{BuildHasher, Hash};

use indexmap_2::IndexSet;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::{ArchivedIndexSet, IndexSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive, S> Archive for IndexSet<K, S> {
    type Archived = ArchivedIndexSet<K::Archived>;
    type Resolver = IndexSetResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedIndexSet::resolve_from_len(self.len(), (7, 8), resolver, out);
    }
}

impl<K, S, RandomState> Serialize<S> for IndexSet<K, RandomState>
where
    K: Hash + Eq + Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<IndexSetResolver, S::Error> {
        ArchivedIndexSet::<K::Archived>::serialize_from_iter::<_, K, _>(
            self.iter(),
            (7, 8),
            serializer,
        )
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
    use core::hash::BuildHasherDefault;

    use indexmap_2::IndexSet;

    use crate::{
        alloc::string::String, api::test::roundtrip_with, hash::FxHasher64,
    };

    #[test]
    fn index_set() {
        let mut value =
            IndexSet::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(String::from("foo"));
        value.insert(String::from("bar"));
        value.insert(String::from("baz"));
        value.insert(String::from("bat"));

        roundtrip_with(&value, |a, b| {
            assert_eq!(a.len(), b.len());
            for k in a.iter() {
                let ak = b.get(k.as_str()).unwrap();
                assert_eq!(k, ak);
            }
        });
    }
}
