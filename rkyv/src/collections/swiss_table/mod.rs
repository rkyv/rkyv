//! SwissTable-based implementation for archived hash map and hash set.

pub mod index_map;
pub mod index_set;
pub mod map;
pub mod set;
pub mod table;

pub use index_map::{ArchivedIndexMap, IndexMapResolver};
pub use index_set::{ArchivedIndexSet, IndexSetResolver};
pub use map::{ArchivedHashMap, HashMapResolver};
use munge::munge;
use rancor::Fallible;
pub use set::{ArchivedHashSet, HashSetResolver};
pub use table::{ArchivedHashTable, HashTableResolver};

use crate::{Archive, Place, Portable, Serialize};

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
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        munge!(let Entry { key, value } = out);
        K::resolve(self.key, resolver.key, key);
        V::resolve(self.value, resolver.value, value);
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

#[derive(Portable)]
#[archive(crate)]
#[repr(C)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
struct Entry<K, V> {
    key: K,
    value: V,
}
