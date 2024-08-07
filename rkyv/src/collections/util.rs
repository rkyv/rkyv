//! Utilities for archived collections.

use core::fmt;
use std::marker::PhantomData;

use munge::munge;
use rancor::Fallible;

use crate::{with::{ArchiveWith, SerializeWith}, Archive, Place, Portable, Serialize};

/// An adapter which serializes and resolves its key and value references.
pub struct EntryAdapter<'a, K, V> {
    /// The key to serialize and resolve.
    pub key: &'a K,
    /// The value to serialize and resolve.
    pub value: &'a V,
}

/// A resolver for a key-value pair.
pub struct EntryResolver<K, V> {
    /// The key resolver.
    pub key: K,
    /// The value resolver.
    pub value: V,
}





impl<K: Archive, V: Archive> Archive for EntryAdapter<'_, K, V> {
    type Archived = Entry<K::Archived, V::Archived>;
    type Resolver = EntryResolver<K::Resolver, V::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
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

/// A key-value entry.
#[derive(Debug, Portable, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[rkyv(crate)]
#[repr(C)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
pub struct Entry<K, V> {
    /// The entry's key.
    pub key: K,
    /// The entry's value.
    pub value: V,
}

/// An error describing that an iterator's length did not match the number of
/// elements it yielded.
#[derive(Debug)]
pub struct IteratorLengthMismatch {
    /// The number of expected elements.
    pub expected: usize,
    /// The actual number of elements.
    pub actual: usize,
}

impl fmt::Display for IteratorLengthMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "iterator claimed that it contained {} elements, but yielded {} \
             items during iteration",
            self.expected, self.actual,
        )
    }
}

#[cfg(feature = "std")]
impl std::error::Error for IteratorLengthMismatch {}
