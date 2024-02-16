//! SwissTable-based implementation for archived hash map and hash set.

pub mod index_map;
pub mod index_set;
pub mod map;
pub mod set;
pub mod table;

pub use index_map::{ArchivedIndexMap, IndexMapResolver};
pub use index_set::{ArchivedIndexSet, IndexSetResolver};
pub use map::{ArchivedHashMap, HashMapResolver};
use rancor::Fallible;
pub use set::{ArchivedHashSet, HashSetResolver};
pub use table::{ArchivedHashTable, HashTableResolver};

use crate::{Archive, Serialize};

struct EntryAdapter<'a, K, V> {
    key: &'a K,
    value: &'a V,
}

struct EntryResolver<K, V> {
    key: K,
    value: V,
}

impl<K: Archive, V: Archive> Archive for EntryAdapter<'_, K, V> {
    type Archived = Entry<K::Archived, V::Archived>;
    type Resolver = EntryResolver<K::Resolver, V::Resolver>;

    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.key);
        K::resolve(self.key, pos + fp, resolver.key, fo);
        let (fp, fo) = out_field!(out.value);
        V::resolve(self.value, pos + fp, resolver.value, fo);
    }
}

impl<S, K, V> Serialize<S> for EntryAdapter<'_, K, V>
where
    S: Fallible + ?Sized,
    K: Serialize<S>,
    V: Serialize<S>,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(EntryResolver {
            key: self.key.serialize(serializer)?,
            value: self.value.serialize(serializer)?,
        })
    }
}

#[cfg_attr(feature = "stable_layout", repr(C))]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
struct Entry<K, V> {
    key: K,
    value: V,
}
