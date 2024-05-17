use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};

use hashbrown::HashSet;
use rancor::{Fallible, Source};

use crate::{
    collections::swiss_table::set::{ArchivedHashSet, HashSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Place, Serialize,
};

impl<K: Archive + Hash + Eq, S> Archive for HashSet<K, S>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashSet<K::Archived>;
    type Resolver = HashSetResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedHashSet::<K::Archived>::resolve_from_len(
            self.len(),
            (7, 8),
            resolver,
            out,
        );
    }
}

impl<K, S, RS> Serialize<S> for HashSet<K, RS>
where
    K::Archived: Hash + Eq,
    K: Serialize<S> + Hash + Eq,
    S: Fallible + Allocator + Writer + ?Sized,
    S::Error: Source,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedHashSet::<K::Archived>::serialize_from_iter(
            self.iter(),
            (7, 8),
            serializer,
        )
    }
}

impl<K, D, S> Deserialize<HashSet<K, S>, D> for ArchivedHashSet<K::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D> + Hash + Eq,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<HashSet<K, S>, D::Error> {
        let mut result = HashSet::with_hasher(S::default());
        for k in self.iter() {
            result.insert(k.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq, S: BuildHasher>
    PartialEq<HashSet<K, S>> for ArchivedHashSet<AK>
{
    fn eq(&self, other: &HashSet<K, S>) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().all(|key| other.get(key).is_some())
        }
    }
}

impl<K: Hash + Eq + Borrow<AK>, AK: Hash + Eq, S: BuildHasher>
    PartialEq<ArchivedHashSet<AK>> for HashSet<K, S>
{
    fn eq(&self, other: &ArchivedHashSet<AK>) -> bool {
        other.eq(self)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    use alloc::string::String;

    use hashbrown::HashSet;

    use crate::test::roundtrip_with;

    #[test]
    fn index_set() {
        let mut value = HashSet::new();
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

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validate_index_set() {
        use rancor::Panic;

        use crate::{
            access, collections::swiss_table::ArchivedHashSet,
            string::ArchivedString, to_bytes,
        };

        let mut value = HashSet::new();
        value.insert(String::from("foo"));
        value.insert(String::from("bar"));
        value.insert(String::from("baz"));
        value.insert(String::from("bat"));

        let bytes = to_bytes::<Panic>(&value).unwrap();
        access::<ArchivedHashSet<ArchivedString>, rancor::Panic>(
            bytes.as_ref(),
        )
        .expect("failed to validate archived index set");
    }
}
