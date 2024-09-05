//! Utilities for archived collections.

use core::{borrow::Borrow, error::Error, fmt, marker::PhantomData};

use munge::munge;
use rancor::Fallible;

use crate::{Archive, Place, Portable, Serialize};

/// An adapter which serializes and resolves its key and value references.
pub struct EntryAdapter<BK, BV, K, V> {
    /// The key to serialize and resolve.
    pub key: BK,
    /// The value to serialize and resolve.
    pub value: BV,

    _phantom: PhantomData<(K, V)>,
}

impl<BK, BV, K, V> EntryAdapter<BK, BV, K, V> {
    /// Returns a new `EntryAdapter` for the given key and value.
    pub fn new(key: BK, value: BV) -> Self {
        Self {
            key,
            value,
            _phantom: PhantomData,
        }
    }
}

/// A resolver for a key-value pair.
pub struct EntryResolver<K, V> {
    /// The key resolver.
    pub key: K,
    /// The value resolver.
    pub value: V,
}

impl<BK, BV, K, V> Archive for EntryAdapter<BK, BV, K, V>
where
    BK: Borrow<K>,
    BV: Borrow<V>,
    K: Archive,
    V: Archive,
{
    type Archived = Entry<K::Archived, V::Archived>;
    type Resolver = EntryResolver<K::Resolver, V::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let Entry { key, value } = out);
        K::resolve(self.key.borrow(), resolver.key, key);
        V::resolve(self.value.borrow(), resolver.value, value);
    }
}

impl<S, BK, BV, K, V> Serialize<S> for EntryAdapter<BK, BV, K, V>
where
    S: Fallible + ?Sized,
    BK: Borrow<K>,
    BV: Borrow<V>,
    K: Serialize<S>,
    V: Serialize<S>,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(EntryResolver {
            key: self.key.borrow().serialize(serializer)?,
            value: self.value.borrow().serialize(serializer)?,
        })
    }
}

/// A key-value entry.
#[derive(Debug, Portable, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
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

impl Error for IteratorLengthMismatch {}
