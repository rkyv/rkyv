//! [`Archive`](crate::Archive) implementation for B-tree sets.

use core::{borrow::Borrow, fmt, ops::ControlFlow};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    collections::btree_map::{ArchivedBTreeMap, BTreeMapResolver},
    ser::{Allocator, Writer},
    Place, Portable, Serialize,
};

/// An archived `BTreeSet`. This is a wrapper around a B-tree map with the same
/// key and a value of `()`.
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Portable)]
#[rkyv(crate)]
#[repr(transparent)]
pub struct ArchivedBTreeSet<K, const E: usize = 5>(ArchivedBTreeMap<K, (), E>);

impl<K, const E: usize> ArchivedBTreeSet<K, E> {
    /// Returns `true` if the set contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the set's key type, but the ordering
    /// on the borrowed form _must_ match the ordering on the key type.
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
    {
        self.0.contains_key(key)
    }

    /// Returns a reference to the value in the set, if any, that is equal to
    /// the given value.
    ///
    /// The value may be any borrowed form of the set's value type, but the
    /// ordering on the borrowed form _must_ match the ordering on the value
    /// type.
    pub fn get<Q: Ord + ?Sized>(&self, value: &Q) -> Option<&K>
    where
        K: Borrow<Q> + Ord,
    {
        self.0.get_key_value(value).map(|(key, _)| key)
    }

    /// Returns `true` if the set contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of items in the archived B-tree set.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Resolves a B-tree set from its length.
    pub fn resolve_from_len(
        len: usize,
        resolver: BTreeSetResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedBTreeSet(inner) = out);
        ArchivedBTreeMap::<K, (), E>::resolve_from_len(len, resolver.0, inner);
    }

    /// Serializes an `ArchivedBTreeSet` from the given iterator and serializer.
    pub fn serialize_from_ordered_iter<I, KU, S>(
        iter: I,
        serializer: &mut S,
    ) -> Result<BTreeSetResolver, S::Error>
    where
        I: ExactSizeIterator,
        I::Item: Borrow<KU>,
        KU: Serialize<S, Archived = K>,
        S: Fallible + Allocator + Writer + ?Sized,
        S::Error: Source,
    {
        ArchivedBTreeMap::<K, (), E>::serialize_from_ordered_iter::<
            _,
            _,
            _,
            _,
            (),
            _,
        >(iter.map(|k| (k, &())), serializer)
        .map(BTreeSetResolver)
    }

    /// Visits every key in the B-tree with a function.
    ///
    /// If `f` returns `ControlFlow::Break`, `visit` will return `Some` with the
    /// broken value. If `f` returns `Continue` for every key in the tree,
    /// `visit` will return `None`.
    pub fn visit<T>(
        &self,
        mut f: impl FnMut(&K) -> ControlFlow<T>,
    ) -> Option<T> {
        self.0.visit(|k, _| f(k))
    }
}

impl<K, const E: usize> fmt::Debug for ArchivedBTreeSet<K, E>
where
    K: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut set = f.debug_set();
        self.visit(|k| {
            set.entry(k);
            ControlFlow::<()>::Continue(())
        });
        set.finish()
    }
}

/// The resolver for archived B-tree sets.
pub struct BTreeSetResolver(BTreeMapResolver);

#[cfg(feature = "alloc")]
mod iter {
    use core::iter::FusedIterator;

    use super::ArchivedBTreeSet;
    use crate::collections::btree_map;

    pub struct Iter<'a, K, const E: usize> {
        inner: btree_map::Keys<'a, K, (), E>,
    }

    impl<'a, K, const E: usize> Iterator for Iter<'a, K, E> {
        type Item = &'a K;

        fn next(&mut self) -> Option<Self::Item> {
            self.inner.next()
        }
    }

    impl<'a, K, const E: usize> ExactSizeIterator for Iter<'a, K, E> {}

    impl<'a, K, const E: usize> FusedIterator for Iter<'a, K, E> {}

    impl<K, const E: usize> ArchivedBTreeSet<K, E> {
        /// Returns an iterator over the items of the archived B-tree set.
        pub fn iter(&self) -> Iter<'_, K, E> {
            Iter {
                inner: self.0.keys(),
            }
        }
    }
}
