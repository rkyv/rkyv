//! [`Archive`] implementation for B-tree maps.

#[cfg(feature = "validation")]
pub mod validation;

use crate::{Archive, ArchivePointee, Archived, RelPtr};
use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    iter::FusedIterator,
    marker::PhantomData,
    ops::{Bound, Index, RangeBounds},
    ptr::NonNull,
};
use ptr_meta::Pointee;

#[cfg_attr(feature = "strict", repr(C))]
struct InnerNodeEntry<K> {
    ptr: RelPtr<NodeHeader>,
    key: K,
}

#[cfg_attr(feature = "strict", repr(C))]
struct LeafNodeEntry<K, V> {
    key: K,
    value: V,
}

impl<'a, UK: Archive, UV: Archive> Archive for LeafNodeEntry<&'a UK, &'a UV> {
    type Archived = LeafNodeEntry<UK::Archived, UV::Archived>;
    type Resolver = (UK::Resolver, UV::Resolver);

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let (fp, fo) = out_field!(out.key);
        self.key.resolve(pos + fp, resolver.0, fo);
        let (fp, fo) = out_field!(out.value);
        self.value.resolve(pos + fp, resolver.1, fo);
    }
}

#[cfg_attr(feature = "strict", repr(C))]
struct NodeHeader {
    meta: Archived<u16>,
    size: Archived<usize>,
    // For leaf nodes, this points to the next leaf node in order
    // For inner nodes, this points to the node in the next layer that's less than the first key in
    // this node
    ptr: RelPtr<NodeHeader>,
}

impl NodeHeader {
    #[inline]
    fn is_inner(&self) -> bool {
        split_meta(from_archived!(self.meta)).0
    }

    #[inline]
    fn is_leaf(&self) -> bool {
        !split_meta(from_archived!(self.meta)).0
    }

    #[inline]
    fn len(&self) -> usize {
        split_meta(from_archived!(self.meta)).1
    }
}

#[inline]
#[cfg(feature = "alloc")]
fn combine_meta(is_inner: bool, len: usize) -> u16 {
    if is_inner {
        0x80_00 | len as u16
    } else {
        len as u16
    }
}

#[inline]
fn split_meta(meta: u16) -> (bool, usize) {
    (meta & 0x80_00 == 0x80_00, (meta & 0x7F_FF) as usize)
}

#[cfg_attr(feature = "strict", repr(C))]
struct Node<T: ?Sized> {
    header: NodeHeader,
    tail: T,
}

impl<T> Pointee for Node<[T]> {
    type Metadata = usize;
}

impl<T> ArchivePointee for Node<[T]> {
    type ArchivedMetadata = Archived<usize>;

    #[inline]
    fn pointer_metadata(archived: &Self::ArchivedMetadata) -> <Self as Pointee>::Metadata {
        from_archived!(*archived) as usize
    }
}

type InnerNode<K> = Node<[InnerNodeEntry<K>]>;
type LeafNode<K, V> = Node<[LeafNodeEntry<K, V>]>;

struct NodeHeaderData {
    meta: u16,
    size: usize,
    pos: Option<usize>,
}

impl Archive for NodeHeaderData {
    type Archived = NodeHeader;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(&self, pos: usize, _: Self::Resolver, out: *mut Self::Archived) {
        let (fp, fo) = out_field!(out.meta);
        self.meta.resolve(pos + fp, (), fo);

        let (fp, fo) = out_field!(out.size);
        self.size.resolve(pos + fp, (), fo);

        let (fp, fo) = out_field!(out.ptr);
        RelPtr::emplace(pos + fp, self.pos.unwrap_or(pos + fp), fo);
    }
}

struct InnerNodeEntryData<'a, UK> {
    key: &'a UK,
}

impl<'a, UK: Archive> Archive for InnerNodeEntryData<'a, UK> {
    type Archived = InnerNodeEntry<UK::Archived>;
    type Resolver = (usize, UK::Resolver);

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        let (fp, fo) = out_field!(out.ptr);
        RelPtr::emplace(pos + fp, resolver.0, fo);
        let (fp, fo) = out_field!(out.key);
        self.key.resolve(pos + fp, resolver.1, fo);
    }
}

enum ClassifiedNode<'a, K, V> {
    Inner(&'a InnerNode<K>),
    Leaf(&'a LeafNode<K, V>),
}

impl NodeHeader {
    #[inline]
    fn classify<K, V>(&self) -> ClassifiedNode<'_, K, V> {
        if self.is_inner() {
            ClassifiedNode::Inner(self.classify_inner())
        } else {
            ClassifiedNode::Leaf(self.classify_leaf())
        }
    }

    #[inline]
    fn classify_inner_ptr<K>(&self) -> *const InnerNode<K> {
        ptr_meta::from_raw_parts(self as *const Self as *const (), self.len())
    }

    #[inline]
    fn classify_inner<K>(&self) -> &'_ InnerNode<K> {
        debug_assert!(self.is_inner());
        unsafe { &*self.classify_inner_ptr() }
    }

    #[inline]
    fn classify_leaf_ptr<K, V>(&self) -> *const LeafNode<K, V> {
        ptr_meta::from_raw_parts(self as *const Self as *const (), self.len())
    }

    #[inline]
    fn classify_leaf<K, V>(&self) -> &'_ LeafNode<K, V> {
        debug_assert!(self.is_leaf());
        unsafe { &*self.classify_leaf_ptr() }
    }
}

/// An archived [`BTreeMap`](std::collections::BTreeMap).
#[cfg_attr(feature = "strict", repr(C))]
pub struct ArchivedBTreeMap<K, V> {
    len: Archived<usize>,
    root: RelPtr<NodeHeader>,
    _phantom: PhantomData<(K, V)>,
}

/// The resolver for an [`ArchivedBTreeMap`].
pub struct BTreeMapResolver {
    root_pos: usize,
}

/// The minimum number of entries to place in a leaf node.
///
/// This value must be greater than 0
pub const MIN_ENTRIES_PER_LEAF_NODE: usize = 1;

/// The minimum number of entries to place in an inner node.
///
/// This value must be greater than 1
pub const MIN_ENTRIES_PER_INNER_NODE: usize = 2;

impl<K, V> ArchivedBTreeMap<K, V> {
    #[inline]
    fn root(&self) -> Option<ClassifiedNode<K, V>> {
        if self.is_empty() {
            None
        } else {
            let root = unsafe { &*self.root.as_ptr() };
            Some(root.classify())
        }
    }

    #[inline]
    fn first(&self) -> Option<(&Node<[LeafNodeEntry<K, V>]>, usize)> {
        let mut node = self.root()?;
        while let ClassifiedNode::Inner(inner) = node {
            let next = unsafe { &*inner.header.ptr.as_ptr() };
            node = next.classify();
        }
        match node {
            ClassifiedNode::Leaf(leaf) => Some((leaf, 0)),
            ClassifiedNode::Inner(_) => unsafe { core::hint::unreachable_unchecked() },
        }
    }

    /// Returns the first key and value in the map.
    #[inline]
    pub fn first_key_value(&self) -> Option<(&K, &V)> {
        self.first().map(key_value_of_node)
    }

    #[inline]
    fn last(&self) -> Option<(&Node<[LeafNodeEntry<K, V>]>, usize)> {
        let mut node = self.root()?;
        while let ClassifiedNode::Inner(inner) = node {
            let next = unsafe { &*inner.tail.last()?.ptr.as_ptr() };
            node = next.classify();
        }
        match node {
            ClassifiedNode::Leaf(leaf) => Some((leaf, leaf.tail.len() - 1)),
            ClassifiedNode::Inner(_) => unsafe { core::hint::unreachable_unchecked() },
        }
    }

    /// Returns the last key and value in the map.
    #[inline]
    pub fn last_key_value(&self) -> Option<(&K, &V)> {
        self.last().map(key_value_of_node)
    }

    /// Returns `true` if the map contains a value for the specified key.
    ///
    /// The key may be any borrowed form of the map's key type, but the ordering on the borrowed
    /// form _must_ match the ordering on the key type.
    #[inline]
    pub fn contains_key<Q: Ord + ?Sized>(&self, key: &Q) -> bool
    where
        K: Borrow<Q> + Ord,
    {
        self.get_key_value(key).is_some()
    }

    /// Returns a reference to the value corresponding to the key.
    ///
    /// The key may be any borrowed form of the map’s key type, but the ordering on the borrowed
    /// form must match the ordering on the key type.
    #[inline]
    pub fn get<Q: Ord + ?Sized>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q> + Ord,
    {
        self.get_key_value(key).map(|(_, v)| v)
    }

    /// Returns the key-value pair corresponding to the supplied key.
    ///
    /// The supplied key may be any borrowed form of the map’s key type, but the ordering on the
    /// borrowed form must match the ordering on the key type.
    pub fn get_key_value<Q: Ord + ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q> + Ord,
    {
        self.get_node(k).map(key_value_of_node)
    }

    fn get_node<Q: Ord + ?Sized>(&self, k: &Q) -> Option<(&Node<[LeafNodeEntry<K, V>]>, usize)>
    where
        K: Borrow<Q> + Ord,
    {
        let mut current = self.root()?;
        loop {
            match current {
                ClassifiedNode::Inner(inner) => {
                    // Binary search for the next node layer
                    let next = match inner
                        .tail
                        .binary_search_by(|probe| probe.key.borrow().cmp(k))
                    {
                        Ok(i) => unsafe { &*inner.tail[i].ptr.as_ptr() },
                        Err(0) => unsafe { &*inner.header.ptr.as_ptr() },
                        Err(i) => unsafe { &*inner.tail[i - 1].ptr.as_ptr() }
                    };
                    current = next.classify();
                }
                ClassifiedNode::Leaf(left) => {
                    // Binary search for the value
                    if let Ok(i) = left
                        .tail
                        .binary_search_by(|probe| probe.key.borrow().cmp(k))
                    {
                        break Some((left, i));
                    } else {
                        break None;
                    }
                }
            }
        }
    }

    /// Returns the next key value pair after the supplied key
    pub fn next_key_value<Q: Ord + ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q> + Ord,
    {
        self.get_next_node(k).map(key_value_of_node)
    }

    fn get_next_node<'a, Q: Ord + ?Sized>(&'a self, k: &Q) -> Option<(&Node<[LeafNodeEntry<K, V>]>, usize)>
    where
        K: Borrow<Q> + Ord,
    {
        let mut current = self.root()?;
        loop {
            match current {
                ClassifiedNode::Inner(inner) => {
                    // Binary search for the next node layer
                    let next = match inner
                        .tail
                        .binary_search_by(|probe| probe.key.borrow().cmp(k))
                    {
                        Ok(i) => unsafe { &*inner.tail[i].ptr.as_ptr() },
                        Err(0) => unsafe { &*inner.header.ptr.as_ptr() },
                        Err(i) => unsafe { &*inner.tail[i - 1].ptr.as_ptr() }
                    };
                    current = next.classify();
                }
                ClassifiedNode::Leaf(leaf) => {
                    let i = leaf
                        .tail
                        .binary_search_by(|probe| probe.key.borrow().cmp(k))
                        .unwrap_or_else(|a| a)
                        .min(leaf.tail.len() - 1);
                    return next_node((leaf, i));
                }
            }
        }
    }

    /// Returns the prev key value pair after the supplied key
    pub fn prev_key_value<Q: Ord + ?Sized>(&self, k: &Q) -> Option<(&K, &V)>
    where
        K: Borrow<Q> + Ord,
    {
        self.get_prev_node(k).map(key_value_of_node)
    }

    fn get_prev_node<'a, Q: Ord + ?Sized>(&'a self, k: &Q) -> Option<(&Node<[LeafNodeEntry<K, V>]>, usize)>
    where
        K: Borrow<Q> + Ord,
    {
        let mut current = self.root()?;
        loop {
            match current {
                ClassifiedNode::Inner(inner) => {
                    // Binary search for the next node layer
                    let next = match inner
                        .tail
                        .binary_search_by(|probe| probe.key.borrow().cmp(k))
                    {
                        Ok(i) => unsafe { &*inner.tail[i].ptr.as_ptr() },
                        Err(0) => unsafe { &*inner.header.ptr.as_ptr() },
                        Err(i) => unsafe { &*inner.tail[i - 1].ptr.as_ptr() }
                    };
                    current = next.classify();
                }
                ClassifiedNode::Leaf(leaf) => {
                    return match leaf
                        .tail
                        .binary_search_by(|probe| probe.key.borrow().cmp(k)) {
                        Ok(i) => prev_node(self, (leaf, i)),
                        Err(0) => prev_node(self, (leaf, 0)),
                        Err(i) => Some((leaf, i - 1)),
                    };
                }
            }
        }
    }

    /// Returns `true` if the map contains no elements.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Gets an iterator over the entries of the map, sorted by key.
    #[inline]
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: RawIter::new_exact(self),
        }
    }

    /// Constructs a double-ended iterator over a sub-range of elements in the map.
    #[inline]
    pub fn range<R>(&self, range: R) -> Iter<'_, K, V>
    where
        K: Ord,
        K: Borrow<K> + Ord,
        R: RangeBounds<K>,
    {
        Iter {
            inner: RawIter::new_from_range(self, range),
        }
    }

    /// Gets an iterator over the keys of the map, in sorted order.
    #[inline]
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: self.iter().inner,
        }
    }

    /// Returns the number of items in the archived B-tree map.
    #[inline]
    pub fn len(&self) -> usize {
        from_archived!(self.len) as usize
    }

    /// Gets an iterator over the values of the map, in order by key.
    #[inline]
    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            inner: self.iter().inner,
        }
    }

    /// Resolves a B-tree map from its length.
    ///
    /// # Safety
    ///
    /// - `len` must be the number of elements that were serialized
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing a B-tree map
    #[inline]
    pub unsafe fn resolve_from_len(
        len: usize,
        pos: usize,
        resolver: BTreeMapResolver,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.len);
        len.resolve(pos + fp, (), fo);

        let (fp, fo) = out_field!(out.root);
        RelPtr::emplace(pos + fp, resolver.root_pos, fo);
    }
}

#[inline]
fn key_value_of_node<K, V>(node: (&Node<[LeafNodeEntry<K, V>]>, usize)) -> (&K, &V) {
    let entry = &node.0.tail[node.1];
    (&entry.key, &entry.value)
}

#[inline]
fn next_node<'a, K, V>(cur: (&'a Node<[LeafNodeEntry<K, V>]>, usize)) -> Option<(&'a Node<[LeafNodeEntry<K, V>]>, usize)> {
    let mut index = cur.1 + 1;
    let mut leaf = cur.0;
    if index == leaf.tail.len() {
        if leaf.header.ptr.is_null() {
            return None;
        }
        index = 0;
        leaf = unsafe {
            let new_leaf: NonNull<NodeHeader> = NonNull::new_unchecked(leaf.header.ptr.as_ptr() as *mut _);
            new_leaf.as_ref().classify_leaf::<K, V>()
        };
    }
    Some((leaf, index))
}

#[inline]
fn prev_node<'a, K, V>(tree: &'a ArchivedBTreeMap<K, V>, cur: (&'a Node<[LeafNodeEntry<K, V>]>, usize)) -> Option<(&'a Node<[LeafNodeEntry<K, V>]>, usize)>
where K: Ord {
    if cur.1 > 0 {
        return Some((cur.0, cur.1 - 1));
    }

    let k = key_value_of_node(cur).0;

    let mut current = tree.root()?;
    loop {
        match current {
            ClassifiedNode::Inner(inner) => {
                let scan = inner
                    .tail
                    .binary_search_by(|probe| probe.key.borrow().cmp(k));

                let mut next = match scan {
                    Ok(i) => unsafe { &*inner.tail[i].ptr.as_ptr() },
                    Err(0) => unsafe { &*inner.header.ptr.as_ptr() },
                    Err(i) => unsafe { &*inner.tail[i - 1].ptr.as_ptr() }
                };

                let mut next_classified: ClassifiedNode<'a, K, V> = next.classify();
                if let ClassifiedNode::Leaf(leaf) = next_classified {
                    if std::ptr::addr_eq(leaf, cur.0) {
                        next = match scan {
                            Ok(0) => unsafe { &*inner.header.ptr.as_ptr() },
                            Ok(i) => unsafe { &*inner.tail[i - 1].ptr.as_ptr() },
                            Err(0) => unsafe { &*inner.header.ptr.as_ptr() },
                            Err(i) => unsafe { &*inner.tail[i - 1].ptr.as_ptr() }
                        };
                        next_classified = next.classify();
                    }
                }

                current = next_classified;
            }
            ClassifiedNode::Leaf(leaf) => {
                if std::ptr::addr_eq(leaf, cur.0) {
                    return None;
                }
                return Some((leaf, leaf.tail.len() - 1));
            }
        }
    }
}

#[cfg(feature = "alloc")]
const _: () = {
    use crate::{ser::Serializer, Serialize};
    #[cfg(not(feature = "std"))]
    use alloc::vec::Vec;
    use core::mem;

    impl<K, V> ArchivedBTreeMap<K, V> {
        /// Serializes an ordered iterator of key-value pairs as a B-tree map.
        ///
        /// # Safety
        ///
        /// - Keys returned by the iterator must be unique
        /// - Keys must be in reverse sorted order from last to first
        pub unsafe fn serialize_from_reverse_iter<'a, UK, UV, S, I>(
            mut iter: I,
            serializer: &mut S,
        ) -> Result<BTreeMapResolver, S::Error>
        where
            UK: 'a + Serialize<S, Archived = K>,
            UV: 'a + Serialize<S, Archived = V>,
            S: Serializer + ?Sized,
            I: ExactSizeIterator<Item = (&'a UK, &'a UV)>,
        {
            if iter.len() == 0 {
                Ok(BTreeMapResolver { root_pos: 0 })
            } else {
                // The memory span of a single node should not exceed 4kb to keep everything within
                // the distance of a single IO page
                const MAX_NODE_SIZE: usize = 4096;

                // The nodes that must go in the next level in reverse order (key, node_pos)
                let mut next_level = Vec::new();
                let mut resolvers = Vec::new();

                while let Some((key, value)) = iter.next() {
                    // Start a new block
                    let block_start_pos = serializer.pos();

                    // Serialize the last entry
                    resolvers.push((
                        key,
                        value,
                        key.serialize(serializer)?,
                        value.serialize(serializer)?,
                    ));

                    loop {
                        // This is an estimate of the block size
                        // It's not exact because there may be padding to align the node and entries
                        // slice
                        let estimated_block_size = serializer.pos() - block_start_pos
                            + mem::size_of::<NodeHeader>()
                            + resolvers.len() * mem::size_of::<LeafNodeEntry<K, V>>();

                        // If we've reached or exceeded the maximum node size and have put enough
                        // entries in this node, then break
                        if estimated_block_size >= MAX_NODE_SIZE
                            && resolvers.len() >= MIN_ENTRIES_PER_LEAF_NODE
                        {
                            break;
                        }

                        if let Some((key, value)) = iter.next() {
                            // Serialize the next entry
                            resolvers.push((
                                key,
                                value,
                                key.serialize(serializer)?,
                                value.serialize(serializer)?,
                            ));
                        } else {
                            break;
                        }
                    }

                    // Finish the current node
                    serializer.align(usize::max(
                        mem::align_of::<NodeHeader>(),
                        mem::align_of::<LeafNodeEntry<K, V>>(),
                    ))?;
                    let raw_node = NodeHeaderData {
                        meta: combine_meta(false, resolvers.len()),
                        size: serializer.pos() - block_start_pos,
                        // The last element of next_level is the next block we're linked to
                        pos: next_level.last().map(|&(_, pos)| pos),
                    };

                    // Add the first key and node position to the next level
                    next_level.push((
                        resolvers.last().unwrap().0,
                        serializer.resolve_aligned(&raw_node, ())?,
                    ));

                    serializer.align_for::<LeafNodeEntry<K, V>>()?;
                    for (key, value, key_resolver, value_resolver) in resolvers.drain(..).rev() {
                        serializer.resolve_aligned(
                            &LeafNodeEntry { key, value },
                            (key_resolver, value_resolver),
                        )?;
                    }
                }

                // Subsequent levels are populated by serializing node keys from the previous level
                // When there's only one node left, that's our root
                let mut current_level = Vec::new();
                let mut resolvers = Vec::new();
                while next_level.len() > 1 {
                    // Our previous next_level becomes our current level, and current_level is
                    // guaranteed to be empty at this point
                    mem::swap(&mut current_level, &mut next_level);

                    let mut iter = current_level.drain(..);
                    while iter.len() > 1 {
                        // Start a new inner block
                        let block_start_pos = serializer.pos();

                        // When we break, we're guaranteed to have at least one node left
                        while iter.len() > 1 {
                            let (key, pos) = iter.next().unwrap();

                            // Serialize the next entry
                            resolvers.push((key, pos, key.serialize(serializer)?));

                            // Estimate the block size
                            let estimated_block_size = serializer.pos() - block_start_pos
                                + mem::size_of::<NodeHeader>()
                                + resolvers.len() * mem::size_of::<InnerNodeEntry<K>>();

                            // If we've reached or exceeded the maximum node size and have put enough
                            // keys in this node, then break
                            if estimated_block_size >= MAX_NODE_SIZE
                                && resolvers.len() >= MIN_ENTRIES_PER_INNER_NODE
                            {
                                break;
                            }
                        }

                        // Three cases here:
                        // 1 entry left: use it as the last key
                        // 2 entries left: serialize the next one and use the last as last to avoid
                        //   putting only one entry in the final block
                        // 3+ entries left: use next as last, next block will contain at least two
                        //   entries

                        if iter.len() == 2 {
                            let (key, pos) = iter.next().unwrap();

                            // Serialize the next entry
                            resolvers.push((key, pos, key.serialize(serializer)?));
                        }

                        // The next item is the first node
                        let (first_key, first_pos) = iter.next().unwrap();

                        // Finish the current node
                        serializer.align(usize::max(
                            mem::align_of::<NodeHeaderData>(),
                            mem::align_of::<InnerNodeEntry<K>>(),
                        ))?;
                        let node_header = NodeHeaderData {
                            meta: combine_meta(true, resolvers.len()),
                            size: serializer.pos() - block_start_pos,
                            // The pos of the first key is used to make the pointer for inner nodes
                            pos: Some(first_pos),
                        };

                        // Add the second key and node position to the next level
                        next_level.push((first_key, serializer.resolve_aligned(&node_header, ())?));

                        serializer.align_for::<InnerNodeEntry<K>>()?;
                        for (key, pos, resolver) in resolvers.drain(..).rev() {
                            let inner_node_data = InnerNodeEntryData::<UK> { key };
                            serializer.resolve_aligned(&inner_node_data, (pos, resolver))?;
                        }
                    }

                    debug_assert!(iter.len() == 0);
                }

                // The root is only node in the final level
                Ok(BTreeMapResolver {
                    root_pos: next_level[0].1,
                })
            }
        }
    }
};

impl<K: fmt::Debug, V: fmt::Debug> fmt::Debug for ArchivedBTreeMap<K, V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<K, Q, V> Index<&'_ Q> for ArchivedBTreeMap<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord + ?Sized,
{
    type Output = V;

    fn index(&self, key: &Q) -> &V {
        self.get(key).unwrap()
    }
}

impl<'a, K, V> IntoIterator for &'a ArchivedBTreeMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<K: Eq, V: Eq> Eq for ArchivedBTreeMap<K, V> {}

impl<K: Hash, V: Hash> Hash for ArchivedBTreeMap<K, V> {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        for pair in self.iter() {
            pair.hash(state);
        }
    }
}

impl<K: Ord, V: Ord> Ord for ArchivedBTreeMap<K, V> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.iter().cmp(other.iter())
    }
}

impl<K: PartialEq, V: PartialEq> PartialEq for ArchivedBTreeMap<K, V> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        if self.len() != other.len() {
            false
        } else {
            self.iter().zip(other.iter()).all(|(a, b)| a.eq(&b))
        }
    }
}

impl<K: PartialOrd, V: PartialOrd> PartialOrd for ArchivedBTreeMap<K, V> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.iter().partial_cmp(other.iter())
    }
}

enum RawIterRun<'a, K, V> {
    RunningExact {
        cur: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
        remaining: usize
    },
    Running {
        first: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
        cur: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
        last: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
    },
    RunningInit {
        first: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
        last: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
    },
    LastOne {
        cur: (&'a Node<[LeafNodeEntry<K, V>]>, usize),
    },
    Finished
}

struct RawIter<'a, K, V> {
    run: RawIterRun<'a, K, V>,
    tree: &'a ArchivedBTreeMap<K, V>,
}

impl<'a, K, V> RawIter<'a, K, V> {
    fn new_exact(tree: &'a ArchivedBTreeMap<K, V>) -> Self {
        let first = match tree.first() {
            Some(a) => a,
            None => return Self {
                run: RawIterRun::Finished,
                tree,
            }
        };
        Self {
            run: RawIterRun::RunningExact { cur: first, remaining: tree.len() },
            tree,
        }
    }

    fn new_from_range<R>(tree: &'a ArchivedBTreeMap<K, V>, range: R) -> Self
    where R: RangeBounds<K>,
          K: Ord {
        let start = match range.start_bound() {
            Bound::Included(start) => tree.get_node(start).or_else(|| tree.get_next_node(start)),
            Bound::Excluded(start) => tree.get_next_node(start),
            Bound::Unbounded => tree.first(),
        };
        let end = match range.end_bound() {
            Bound::Included(end) => tree.get_node(end).or_else(|| tree.get_prev_node(end)),
            Bound::Excluded(end) => tree.get_prev_node(end),
            Bound::Unbounded => tree.last(),
        };

        // if either the start or end is null then its already at the end
        let start = match start {
            Some(start) => start,
            None => return Self {
                run: RawIterRun::Finished,
                tree,
            }
        };
        let end = match end {
            Some(end) => end,
            None => return Self {
                run: RawIterRun::Finished,
                tree,
            }
        };
        if key_value_of_node(start).0 > key_value_of_node(end).0 {
            return Self {
                run: RawIterRun::Finished,
                tree,
            };
        }

        // Return the iterator
        Self {
            run: RawIterRun::RunningInit { first: start, last: end },
            tree,
        }
    }
}

impl<'a, K, V> Iterator for RawIter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            return match &mut self.run {
                RawIterRun::RunningExact { cur, remaining } => {
                    let ret = key_value_of_node(*cur);
                    *remaining -= 1;
                    if *remaining == 0 {
                        self.run = RawIterRun::Finished;
                        return Some(ret);
                    }
                    *cur = match next_node(*cur) {
                        Some(a) => a,
                        None => {
                            self.run = RawIterRun::Finished;
                            return Some(ret);
                        }
                    };
                    Some(ret)
                },
                RawIterRun::Running { cur, first, last } => {
                    let ret = key_value_of_node(*cur);
                    *cur = match next_node(*cur) {
                        Some(a) => a,
                        None => *first,
                    };
                    if std::ptr::addr_eq(cur.0, last.0) && cur.1 == last.1 {
                        self.run = RawIterRun::LastOne { cur: *cur };
                    }
                    Some(ret)
                },
                RawIterRun::RunningInit { first, last } => {
                    self.run = RawIterRun::Running { first: *first, cur: *first, last: *last };
                    continue;
                },
                RawIterRun::LastOne { cur } => {
                    let ret = key_value_of_node(*cur);
                    self.run = RawIterRun::Finished;
                    Some(ret)
                },
                RawIterRun::Finished => None,
            };
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.run {
            RawIterRun::RunningExact { remaining, .. } => (*remaining, Some(*remaining)),
            RawIterRun::Running { .. } | RawIterRun::RunningInit { .. } => (0, None),
            RawIterRun::LastOne { .. } => (1, Some(1)),
            RawIterRun::Finished => (0, Some(0)),
        }
    }
}

impl<'a, K, V> DoubleEndedIterator for RawIter<'a, K, V>
where K: Ord {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            return match &mut self.run {
                RawIterRun::RunningExact { cur, .. } => {
                    match (self.tree.first(), self.tree.last()) {
                        (Some(first), Some(last)) if std::ptr::addr_eq(cur.0, first.0) && cur.1 == first.1 => {
                            self.run = RawIterRun::Running { first, cur: last, last };
                            continue;
                        },
                        (Some(first), Some(last)) => {
                            self.run = RawIterRun::Running { first, cur: *cur, last };
                            continue;
                        },
                        _ => {
                            self.run = RawIterRun::Finished;
                            None
                        },
                    }
                },
                RawIterRun::Running { cur, first, last } => {
                    let ret = key_value_of_node(*cur);
                    *cur = match prev_node(self.tree, *cur) {
                        Some(a) => a,
                        None => *last,
                    };
                    if std::ptr::addr_eq(cur.0, first.0) && cur.1 == first.1 {
                        self.run = RawIterRun::LastOne { cur: *cur };
                    }
                    Some(ret)
                },
                RawIterRun::RunningInit { first, last } => {
                    self.run = RawIterRun::Running { first: *first, cur: *last, last: *last };
                    continue;
                },
                RawIterRun::LastOne { cur } => {
                    let ret = key_value_of_node(*cur);
                    self.run = RawIterRun::Finished;
                    Some(ret)
                },
                RawIterRun::Finished => None,
            };
        }
    }
}

impl<'a, K, V> FusedIterator for RawIter<'a, K, V> {}

/// An iterator over the key-value pairs of an archived B-tree map.
pub struct Iter<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}


impl<'a, K, V> DoubleEndedIterator for Iter<'a, K, V>
where K: Ord {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back()
    }
}

impl<'a, K, V> FusedIterator for Iter<'a, K, V> {}

/// An iterator over the keys of an archived B-tree map.
pub struct Keys<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Keys<'a, K, V> {
    type Item = &'a K;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| k)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> DoubleEndedIterator for Keys<'a, K, V>
where K: Ord {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(k, _)| k)
    }
}

impl<'a, K, V> FusedIterator for Keys<'a, K, V> {}

/// An iterator over the values of an archived B-tree map.
pub struct Values<'a, K, V> {
    inner: RawIter<'a, K, V>,
}

impl<'a, K, V> Iterator for Values<'a, K, V> {
    type Item = &'a V;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| v)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<'a, K, V> DoubleEndedIterator for Values<'a, K, V>
where K: Ord {
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|(_, v)| v)
    }
}

impl<'a, K, V> FusedIterator for Values<'a, K, V> {}
