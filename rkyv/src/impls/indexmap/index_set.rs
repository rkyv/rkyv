use core::hash::{BuildHasher, Hash};

use indexmap::IndexSet;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::{ArchivedIndexSet, IndexSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive, S> Archive for IndexSet<K, S> {
    type Archived = ArchivedIndexSet<K::Archived>;
    type Resolver = IndexSetResolver;

    unsafe fn resolve(
        &self,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
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
        ArchivedIndexSet::<K::Archived>::serialize_from_iter(
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
    #[cfg(not(feature = "std"))]
    use alloc::string::String;
    use core::hash::BuildHasherDefault;

    use indexmap::IndexSet;
    use rancor::{Error, Infallible};

    use crate::{
        access_unchecked, collections::swiss_table::ArchivedIndexSet,
        deserialize, hash::FxHasher64, string::ArchivedString,
    };

    #[test]
    fn index_set() {
        let mut value =
            IndexSet::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(String::from("foo"));
        value.insert(String::from("bar"));
        value.insert(String::from("baz"));
        value.insert(String::from("bat"));

        let result = crate::to_bytes::<Error>(&value).unwrap();
        let archived = unsafe {
            access_unchecked::<ArchivedIndexSet<ArchivedString>>(
                result.as_ref(),
            )
        };

        assert_eq!(value.len(), archived.len());
        for k in value.iter() {
            let ak = archived.get(k.as_str()).unwrap();
            assert_eq!(k, ak);
        }

        let deserialized = deserialize::<
            IndexSet<String, BuildHasherDefault<FxHasher64>>,
            _,
            Infallible,
        >(archived, &mut ())
        .unwrap();
        assert_eq!(value, deserialized);
    }

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validate_index_set() {
        use crate::access;

        let mut value =
            IndexSet::with_hasher(BuildHasherDefault::<FxHasher64>::default());
        value.insert(String::from("foo"));
        value.insert(String::from("bar"));
        value.insert(String::from("baz"));
        value.insert(String::from("bat"));

        let result = crate::to_bytes::<Error>(&value).unwrap();
        access::<ArchivedIndexSet<ArchivedString>, Error>(result.as_ref())
            .expect("failed to validate archived index set");
    }
}
