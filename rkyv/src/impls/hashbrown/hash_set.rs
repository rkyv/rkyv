use core::{
    borrow::Borrow,
    hash::{BuildHasher, Hash},
};

use hashbrown::HashSet;
use rancor::{Error, Fallible};

use crate::{
    collections::swiss_table::set::{ArchivedHashSet, HashSetResolver},
    ser::{Allocator, Writer},
    Archive, Deserialize, Serialize,
};

impl<K: Archive + Hash + Eq, S> Archive for HashSet<K, S>
where
    K::Archived: Hash + Eq,
{
    type Archived = ArchivedHashSet<K::Archived>;
    type Resolver = HashSetResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedHashSet::<K::Archived>::resolve_from_len(
            self.len(),
            (7, 8),
            pos,
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
    S::Error: Error,
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedHashSet::serialize_from_iter(self.iter(), (7, 8), serializer)
    }
}

impl<K, D, S> Deserialize<HashSet<K, S>, D> for ArchivedHashSet<K::Archived>
where
    K: Archive + Hash + Eq,
    K::Archived: Deserialize<K, D> + Hash + Eq,
    D: Fallible + ?Sized,
    S: Default + BuildHasher,
{
    #[inline]
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
    #[inline]
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
    #[inline]
    fn eq(&self, other: &ArchivedHashSet<AK>) -> bool {
        other.eq(self)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    use alloc::string::String;

    use hashbrown::HashSet;
    use rancor::Failure;

    use crate::{access, access_unchecked, deserialize, to_bytes};

    #[test]
    fn index_set() {
        let mut value = HashSet::new();
        value.insert(String::from("foo"));
        value.insert(String::from("bar"));
        value.insert(String::from("baz"));
        value.insert(String::from("bat"));

        let bytes = to_bytes::<_, 256, Failure>(&value).unwrap();
        let archived =
            unsafe { access_unchecked::<HashSet<String>>(bytes.as_ref()) };

        assert_eq!(value.len(), archived.len());
        for k in value.iter() {
            let ak = archived.get(k.as_str()).unwrap();
            assert_eq!(k, ak);
        }

        let deserialized =
            deserialize::<HashSet<String>, _, Failure>(archived, &mut ())
                .unwrap();
        assert_eq!(value, deserialized);
    }

    #[cfg(feature = "bytecheck")]
    #[test]
    fn validate_index_set() {
        let mut value = HashSet::new();
        value.insert(String::from("foo"));
        value.insert(String::from("bar"));
        value.insert(String::from("baz"));
        value.insert(String::from("bat"));

        let bytes = to_bytes::<_, 256, Failure>(&value).unwrap();
        access::<HashSet<String>, rancor::Panic>(bytes.as_ref())
            .expect("failed to validate archived index set");
    }
}
