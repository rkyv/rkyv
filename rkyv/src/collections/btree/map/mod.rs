//! [`Archive`](crate::Archive) implementation for B-tree maps.

use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    marker::PhantomData,
    mem::{size_of, MaybeUninit},
    ops::{ControlFlow, Index},
    ptr::addr_of_mut,
    slice,
};

use munge::munge;
use rancor::{fail, Fallible, Source};

use crate::{
    collections::util::IteratorLengthMismatch,
    primitive::{ArchivedUsize, FixedUsize},
    seal::Seal,
    ser::{Allocator, Writer, WriterExt as _},
    traits::NoUndef,
    util::{InlineVec, SerVec},
    Place, Portable, RelPtr, Serialize,
};

// TODO(#515): Get Iterator APIs working without the `alloc` feature enabled
#[cfg(feature = "alloc")]
mod iter;

#[cfg(feature = "alloc")]
pub use self::iter::*;

// B-trees are typically characterized as having a branching factor of B.
// However, in this implementation our B-trees are characterized as having a
// number of entries per node E where E = B - 1. This is done because it's
// easier to add an additional node pointer to each inner node than it is to
// store one less entry per inner node. Because generic const exprs are not
// stable, we can't declare a field `entries: [Entry; { B - 1 }]`. But we can
// declare `branches: [RelPtr; E]` and then add another `last: RelPtr`
// field. When the branching factor B is needed, it will be calculated as E + 1.

const fn nodes_in_level<const E: usize>(i: u32) -> usize {
    // The root of the tree has one node, and each level down has B times as
    // many nodes at the last. Therefore, the number of nodes in the I-th level
    // is equal to B^I.

    (E + 1).pow(i)
}

const fn entries_in_full_tree<const E: usize>(h: u32) -> usize {
    // The number of nodes in each layer I of a B-tree is equal to B^I. At layer
    // I = 0, the number of nodes is exactly one. At layer I = 1, the number of
    // nodes is B, at layer I = 2 the number of nodes is B^2, and so on. The
    // total number of nodes is equal to the sum from 0 to H - 1 of B^I. Since
    // this is the sum of a geometric progression, we have the closed-form
    // solution N = (B^H - 1) / (B - 1). Since the number of entries per node is
    // equal to B - 1, we thus have the solution that the number of entries in a
    // B-tree of height H is equal to B^H - 1.

    // Note that this is one less than the number of nodes in the level after
    // the final level of the B-tree.

    nodes_in_level::<E>(h) - 1
}

const fn entries_to_height<const E: usize>(n: usize) -> u32 {
    // Solving B^H - 1 = N for H yields H = log_B(N + 1). However, we'll be
    // using an integer logarithm, and so the value of H will be rounded down
    // which underestimates the height of the tree:
    // => H = ilog_B(N + 1) = floor(log_B(N + 1)).
    // To compensate for this, we'll calculate the height for a tree with a
    // greater number of nodes and choose this greater number so that rounding
    // down will always yield the correct result.

    // The minimum value which yields a height of H is exactly B^H - 1, so we
    // need to add a large enough correction to always be greater than or equal
    // to that value. The maximum value which yields a height of H is one less
    // than the number of nodes in the next-largest B-tree, which is equal to
    // B^(H + 1) - 1. This gives the following relationships for N:
    // => B^(H - 1) - 1 < N <= B^H - 1
    // And the desired relationships for the corrected number of entries C(N):
    // => B^H - 1 <= C(N) < B^(H + 1) - 1

    // First, we can add 1 to the two ends of our first set of relationships
    // to change whether equality is allowed. We can do this because all entries
    // are integers. This makes the relationships match the desired
    // relationships for C(N):
    // => B^(H - 1) - 1 + 1 <= N < B^H - 1 + 1
    // => B^(H - 1) <= N < B^H
    // Let's choose a function to map the lower bound for N to the desired lower
    // bound for C(N):
    // => C(B^(H - 1)) = B^(H - 1)
    // A straightforward choice would be C(N) = B * N - 1. Substituting yields:
    // => C(B^(H - 1)) <= C(N) < C(B^H)
    // => B * B^(H - 1) - 1 <= B * N - 1 < B * B^H - 1
    // => B^H - 1 <= B * N - 1 < B^(H + 1) - 1
    // These exactly match the desired bounds, so this is the function we want.

    // Putting it all together:
    // => H = ilog_B(C(N) + 1) = ilog_b(B * N - 1 + 1) = ilog_b(B * N)
    // => H = 1 + ilog_b(N)
    1 + n.ilog(E + 1)
}

const fn ll_entries<const E: usize>(height: u32, n: usize) -> usize {
    // The number of entries not in the last level is equal to the number of
    // entries in a full B-tree of height H - 1. The number of entries in
    // the last level is thus the total number of entries minus the number
    // of entries not in the last level.
    n - entries_in_full_tree::<E>(height - 1)
}

#[derive(Clone, Copy, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(u8)]
enum NodeKind {
    Leaf,
    Inner,
}

// SAFETY: `NodeKind` is `repr(u8)` and so always consists of a single
// well-defined byte.
unsafe impl NoUndef for NodeKind {}

#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
struct Node<K, V, const E: usize> {
    kind: NodeKind,
    keys: [MaybeUninit<K>; E],
    values: [MaybeUninit<V>; E],
}

#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
struct LeafNode<K, V, const E: usize> {
    node: Node<K, V, E>,
    len: ArchivedUsize,
}

#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
struct InnerNode<K, V, const E: usize> {
    node: Node<K, V, E>,
    lesser_nodes: [RelPtr<Node<K, V, E>>; E],
    greater_node: RelPtr<Node<K, V, E>>,
}

/// An archived [`BTreeMap`](crate::alloc::collections::BTreeMap).
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedBTreeMap<K, V, const E: usize = 5> {
    // The type of the root node is determined at runtime because it may point
    // to:
    // - Nothing if the length is zero
    // - A leaf node if there is only one node
    // - Or an inner node if there are multiple nodes
    root: RelPtr<Node<K, V, E>>,
    len: ArchivedUsize,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V, const E: usize> ArchivedBTreeMap<K, V, E> {
    /// Returns whether the B-tree map contains the given key.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        self.get_key_value(key).is_some()
    }

    /// Returns the value associated with the given key, or `None` if the key is
    /// not present in the B-tree map.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        Some(self.get_key_value(key)?.1)
    }

    /// Returns the mutable value associated with the given key, or `None` if
    /// the key is not present in the B-tree map.
    pub fn get_seal<'a, Q>(this: Seal<'a, Self>, key: &Q) -> Option<Seal<'a, V>>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        Some(Self::get_key_value_seal(this, key)?.1)
    }

    /// Returns true if the B-tree map contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of entries in the B-tree map.
    pub fn len(&self) -> usize {
        self.len.to_native() as usize
    }

    /// Gets the key-value pair associated with the given key, or `None` if the
    /// key is not present in the B-tree map.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        self.get_key_value_with(key, |q, k| q.cmp(k.borrow()))
    }

    /// Gets the key-value pair associated with the given key, or `None` if the
    /// key is not present in the B-tree map.
    ///
    /// This method uses the supplied comparison function to compare the key to
    /// elements.
    pub fn get_key_value_with<Q, C>(&self, key: &Q, cmp: C) -> Option<(&K, &V)>
    where
        Q: Ord + ?Sized,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let this = (self as *const Self).cast_mut();
        Self::get_key_value_raw(this, key, cmp)
            .map(|(k, v)| (unsafe { &*k }, unsafe { &*v }))
    }

    /// Gets the mutable key-value pair associated with the given key, or `None`
    /// if the key is not present in the B-tree map.
    pub fn get_key_value_seal<'a, Q>(
        this: Seal<'a, Self>,
        key: &Q,
    ) -> Option<(&'a K, Seal<'a, V>)>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        Self::get_key_value_seal_with(this, key, |q, k| q.cmp(k.borrow()))
    }

    /// Gets the mutable key-value pair associated with the given key, or `None`
    /// if the key is not present in the B-tree map.
    ///
    /// This method uses the supplied comparison function to compare the key to
    /// elements.
    pub fn get_key_value_seal_with<'a, Q, C>(
        this: Seal<'a, Self>,
        key: &Q,
        cmp: C,
    ) -> Option<(&'a K, Seal<'a, V>)>
    where
        Q: Ord + ?Sized,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let this = unsafe { Seal::unseal_unchecked(this) as *mut Self };
        Self::get_key_value_raw(this, key, cmp)
            .map(|(k, v)| (unsafe { &*k }, Seal::new(unsafe { &mut *v })))
    }

    fn get_key_value_raw<Q, C>(
        this: *mut Self,
        key: &Q,
        cmp: C,
    ) -> Option<(*mut K, *mut V)>
    where
        Q: Ord + ?Sized,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let len = unsafe { (*this).len.to_native() };
        if len == 0 {
            return None;
        }

        let root_ptr = unsafe { addr_of_mut!((*this).root) };
        let mut current = unsafe { RelPtr::as_ptr_raw(root_ptr) };
        'outer: loop {
            let kind = unsafe { (*current).kind };

            match kind {
                NodeKind::Leaf => {
                    let leaf = current.cast::<LeafNode<K, V, E>>();
                    let len = unsafe { (*leaf).len };

                    for i in 0..len.to_native() as usize {
                        let k = unsafe {
                            addr_of_mut!((*current).keys[i]).cast::<K>()
                        };
                        let ordering = cmp(key, unsafe { &*k });

                        match ordering {
                            Ordering::Equal => {
                                let v = unsafe {
                                    addr_of_mut!((*current).values[i])
                                        .cast::<V>()
                                };
                                return Some((k, v));
                            }
                            Ordering::Less => return None,
                            Ordering::Greater => (),
                        }
                    }

                    return None;
                }
                NodeKind::Inner => {
                    let inner = current.cast::<InnerNode<K, V, E>>();

                    for i in 0..E {
                        let k = unsafe {
                            addr_of_mut!((*current).keys[i]).cast::<K>()
                        };
                        let ordering = cmp(key, unsafe { &*k });

                        match ordering {
                            Ordering::Equal => {
                                let v = unsafe {
                                    addr_of_mut!((*current).values[i])
                                        .cast::<V>()
                                };
                                return Some((k, v));
                            }
                            Ordering::Less => {
                                let lesser = unsafe {
                                    addr_of_mut!((*inner).lesser_nodes[i])
                                };
                                let lesser_is_invalid =
                                    unsafe { RelPtr::is_invalid_raw(lesser) };
                                if !lesser_is_invalid {
                                    current =
                                        unsafe { RelPtr::as_ptr_raw(lesser) };
                                    continue 'outer;
                                } else {
                                    return None;
                                }
                            }
                            Ordering::Greater => (),
                        }
                    }

                    let inner = current.cast::<InnerNode<K, V, E>>();
                    let greater =
                        unsafe { addr_of_mut!((*inner).greater_node) };
                    let greater_is_invalid =
                        unsafe { RelPtr::is_invalid_raw(greater) };
                    if !greater_is_invalid {
                        current = unsafe { RelPtr::as_ptr_raw(greater) };
                    } else {
                        return None;
                    }
                }
            }
        }
    }

    /// Resolves an `ArchivedBTreeMap` from the given length, resolver, and
    /// output place.
    pub fn resolve_from_len(
        len: usize,
        resolver: BTreeMapResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedBTreeMap { root, len: out_len, _phantom: _ } = out);

        if len == 0 {
            RelPtr::emplace_invalid(root);
        } else {
            RelPtr::emplace(resolver.root_node_pos as usize, root);
        }

        out_len.write(ArchivedUsize::from_native(len as FixedUsize));
    }

    /// Serializes an `ArchivedBTreeMap` from the given iterator and serializer.
    pub fn serialize_from_ordered_iter<I, BKU, BVU, KU, VU, S>(
        mut iter: I,
        serializer: &mut S,
    ) -> Result<BTreeMapResolver, S::Error>
    where
        I: ExactSizeIterator<Item = (BKU, BVU)>,
        BKU: Borrow<KU>,
        BVU: Borrow<VU>,
        KU: Serialize<S, Archived = K>,
        VU: Serialize<S, Archived = V>,
        S: Fallible + Allocator + Writer + ?Sized,
        S::Error: Source,
    {
        let len = iter.len();

        if len == 0 {
            let actual = iter.count();
            if actual != 0 {
                fail!(IteratorLengthMismatch {
                    expected: 0,
                    actual,
                });
            }
            return Ok(BTreeMapResolver { root_node_pos: 0 });
        }

        let height = entries_to_height::<E>(len);
        let ll_entries = ll_entries::<E>(height, len);

        SerVec::with_capacity(
            serializer,
            height as usize - 1,
            |open_inners, serializer| {
                for _ in 0..height - 1 {
                    open_inners
                        .push(InlineVec::<(BKU, BVU, Option<usize>), E>::new());
                }

                let mut open_leaf = InlineVec::<(BKU, BVU), E>::new();

                let mut child_node_pos = None;
                let mut leaf_entries = 0;
                while let Some((key, value)) = iter.next() {
                    open_leaf.push((key, value));
                    leaf_entries += 1;

                    if leaf_entries == ll_entries
                        || open_leaf.len() == open_leaf.capacity()
                    {
                        // Close open leaf
                        child_node_pos =
                            Some(Self::close_leaf(&open_leaf, serializer)?);
                        open_leaf.clear();

                        // If on the transition node, fill and close open inner
                        if leaf_entries == ll_entries {
                            if let Some(mut inner) = open_inners.pop() {
                                while inner.len() < inner.capacity() {
                                    if let Some((k, v)) = iter.next() {
                                        inner.push((k, v, child_node_pos));
                                        child_node_pos = None;
                                    } else {
                                        break;
                                    }
                                }

                                child_node_pos = Some(Self::close_inner(
                                    &inner,
                                    child_node_pos,
                                    serializer,
                                )?);
                            }
                        }

                        // Add closed node to open inner
                        let mut popped = 0;
                        while let Some(last_inner) = open_inners.last_mut() {
                            if last_inner.len() == last_inner.capacity() {
                                // Close open inner
                                child_node_pos = Some(Self::close_inner(
                                    last_inner,
                                    child_node_pos,
                                    serializer,
                                )?);
                                open_inners.pop();
                                popped += 1;
                            } else {
                                let (key, value) = iter.next().unwrap();
                                last_inner.push((key, value, child_node_pos));
                                child_node_pos = None;
                                for _ in 0..popped {
                                    open_inners.push(InlineVec::default());
                                }
                                break;
                            }
                        }
                    }
                }

                if !open_leaf.is_empty() {
                    // Close open leaf
                    child_node_pos =
                        Some(Self::close_leaf(&open_leaf, serializer)?);
                    open_leaf.clear();
                }

                // Close open inners
                while let Some(inner) = open_inners.pop() {
                    child_node_pos = Some(Self::close_inner(
                        &inner,
                        child_node_pos,
                        serializer,
                    )?);
                }

                debug_assert!(open_inners.is_empty());
                debug_assert!(open_leaf.is_empty());

                let leftovers = iter.count();
                if leftovers != 0 {
                    fail!(IteratorLengthMismatch {
                        expected: len,
                        actual: len + leftovers,
                    });
                }

                Ok(BTreeMapResolver {
                    root_node_pos: child_node_pos.unwrap() as FixedUsize,
                })
            },
        )?
    }

    fn close_leaf<BKU, BVU, KU, VU, S>(
        items: &[(BKU, BVU)],
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        BKU: Borrow<KU>,
        BVU: Borrow<VU>,
        KU: Serialize<S, Archived = K>,
        VU: Serialize<S, Archived = V>,
        S: Writer + Fallible + ?Sized,
    {
        let mut resolvers = InlineVec::<(KU::Resolver, VU::Resolver), E>::new();
        for (key, value) in items {
            resolvers.push((
                key.borrow().serialize(serializer)?,
                value.borrow().serialize(serializer)?,
            ));
        }

        let pos = serializer.align_for::<LeafNode<K, V, E>>()?;
        let mut node = MaybeUninit::<LeafNode<K, V, E>>::uninit();
        // SAFETY: `node` is properly aligned and valid for writes of
        // `size_of::<LeafNode<K, V, E>>()` bytes.
        unsafe {
            node.as_mut_ptr().write_bytes(0, 1);
        }

        let node_place =
            unsafe { Place::new_unchecked(pos, node.as_mut_ptr()) };

        munge! {
            let LeafNode {
                node: Node {
                    kind,
                    keys,
                    values,
                },
                len,
            } = node_place;
        }
        kind.write(NodeKind::Leaf);
        len.write(ArchivedUsize::from_native(items.len() as FixedUsize));
        for (i, ((k, v), (kr, vr))) in
            items.iter().zip(resolvers.drain()).enumerate()
        {
            let out_key = unsafe { keys.index(i).cast_unchecked() };
            k.borrow().resolve(kr, out_key);
            let out_value = unsafe { values.index(i).cast_unchecked() };
            v.borrow().resolve(vr, out_value);
        }

        let bytes = unsafe {
            slice::from_raw_parts(
                node.as_ptr().cast::<u8>(),
                size_of::<LeafNode<K, V, E>>(),
            )
        };
        serializer.write(bytes)?;

        Ok(pos)
    }

    fn close_inner<BKU, BVU, KU, VU, S>(
        items: &[(BKU, BVU, Option<usize>)],
        greater_node_pos: Option<usize>,
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        BKU: Borrow<KU>,
        BVU: Borrow<VU>,
        KU: Serialize<S, Archived = K>,
        VU: Serialize<S, Archived = V>,
        S: Writer + Fallible + ?Sized,
    {
        debug_assert_eq!(items.len(), E);

        let mut resolvers = InlineVec::<(KU::Resolver, VU::Resolver), E>::new();
        for (key, value, _) in items {
            resolvers.push((
                key.borrow().serialize(serializer)?,
                value.borrow().serialize(serializer)?,
            ));
        }

        let pos = serializer.align_for::<InnerNode<K, V, E>>()?;
        let mut node = MaybeUninit::<InnerNode<K, V, E>>::uninit();
        // SAFETY: `node` is properly aligned and valid for writes of
        // `size_of::<InnerNode<K, V, E>>()` bytes.
        unsafe {
            node.as_mut_ptr().write_bytes(0, 1);
        }

        let node_place =
            unsafe { Place::new_unchecked(pos, node.as_mut_ptr()) };

        munge! {
            let InnerNode {
                node: Node {
                    kind,
                    keys,
                    values,
                },
                lesser_nodes,
                greater_node,
            } = node_place;
        }

        kind.write(NodeKind::Inner);
        for (i, ((k, v, l), (kr, vr))) in
            items.iter().zip(resolvers.drain()).enumerate()
        {
            let out_key = unsafe { keys.index(i).cast_unchecked() };
            k.borrow().resolve(kr, out_key);
            let out_value = unsafe { values.index(i).cast_unchecked() };
            v.borrow().resolve(vr, out_value);

            let out_lesser_node = unsafe { lesser_nodes.index(i) };
            if let Some(lesser_node) = l {
                RelPtr::emplace(*lesser_node, out_lesser_node);
            } else {
                RelPtr::emplace_invalid(out_lesser_node);
            }
        }

        if let Some(greater_node_pos) = greater_node_pos {
            RelPtr::emplace(greater_node_pos, greater_node);
        } else {
            RelPtr::emplace_invalid(greater_node);
        }

        let bytes = unsafe {
            slice::from_raw_parts(
                node.as_ptr().cast::<u8>(),
                size_of::<InnerNode<K, V, E>>(),
            )
        };
        serializer.write(bytes)?;

        Ok(pos)
    }

    /// Visits every key-value pair in the B-tree with a function.
    ///
    /// If `f` returns `ControlFlow::Break`, `visit` will return `Some` with the
    /// broken value. If `f` returns `Continue` for every pair in the tree,
    /// `visit` will return `None`.
    pub fn visit<T>(
        &self,
        mut f: impl FnMut(&K, &V) -> ControlFlow<T>,
    ) -> Option<T> {
        if self.is_empty() {
            None
        } else {
            let root = &self.root;
            let root_ptr = unsafe { root.as_ptr().cast::<Node<K, V, E>>() };
            let mut call_inner = |k: *mut K, v: *mut V| unsafe { f(&*k, &*v) };
            match Self::visit_raw(root_ptr.cast_mut(), &mut call_inner) {
                ControlFlow::Continue(()) => None,
                ControlFlow::Break(x) => Some(x),
            }
        }
    }

    /// Visits every mutable key-value pair in the B-tree with a function.
    ///
    /// If `f` returns `ControlFlow::Break`, `visit` will return `Some` with the
    /// broken value. If `f` returns `Continue` for every pair in the tree,
    /// `visit` will return `None`.
    pub fn visit_seal<T>(
        this: Seal<'_, Self>,
        mut f: impl FnMut(&K, Seal<'_, V>) -> ControlFlow<T>,
    ) -> Option<T> {
        if this.is_empty() {
            None
        } else {
            munge!(let Self { root, .. } = this);
            let root_ptr =
                unsafe { RelPtr::as_mut_ptr(root).cast::<Node<K, V, E>>() };
            let mut call_inner =
                |k: *mut K, v: *mut V| unsafe { f(&*k, Seal::new(&mut *v)) };
            match Self::visit_raw(root_ptr, &mut call_inner) {
                ControlFlow::Continue(()) => None,
                ControlFlow::Break(x) => Some(x),
            }
        }
    }

    fn visit_raw<T>(
        current: *mut Node<K, V, E>,
        f: &mut impl FnMut(*mut K, *mut V) -> ControlFlow<T>,
    ) -> ControlFlow<T> {
        let kind = unsafe { (*current).kind };

        match kind {
            NodeKind::Leaf => {
                let leaf = current.cast::<LeafNode<K, V, E>>();
                let len = unsafe { (*leaf).len };
                for i in 0..len.to_native() as usize {
                    Self::visit_key_value_raw(current, i, f)?;
                }
            }
            NodeKind::Inner => {
                let inner = current.cast::<InnerNode<K, V, E>>();

                // Visit lesser nodes and key-value pairs
                for i in 0..E {
                    let lesser =
                        unsafe { addr_of_mut!((*inner).lesser_nodes[i]) };
                    let lesser_is_invalid =
                        unsafe { RelPtr::is_invalid_raw(lesser) };
                    if !lesser_is_invalid {
                        let lesser_ptr = unsafe { RelPtr::as_ptr_raw(lesser) };
                        Self::visit_raw(lesser_ptr, f)?;
                    }
                    Self::visit_key_value_raw(current, i, f)?;
                }

                // Visit greater node
                let greater = unsafe { addr_of_mut!((*inner).greater_node) };
                let greater_is_invalid =
                    unsafe { RelPtr::is_invalid_raw(greater) };
                if !greater_is_invalid {
                    let greater_ptr = unsafe {
                        RelPtr::as_ptr_raw(greater).cast::<Node<K, V, E>>()
                    };
                    Self::visit_raw(greater_ptr, f)?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    fn visit_key_value_raw<T>(
        current: *mut Node<K, V, E>,
        i: usize,
        f: &mut impl FnMut(*mut K, *mut V) -> ControlFlow<T>,
    ) -> ControlFlow<T> {
        let key_ptr = unsafe { addr_of_mut!((*current).keys[i]).cast::<K>() };
        let value_ptr =
            unsafe { addr_of_mut!((*current).values[i]).cast::<V>() };
        f(key_ptr, value_ptr)
    }
}

impl<K, V, const E: usize> fmt::Debug for ArchivedBTreeMap<K, V, E>
where
    K: fmt::Debug,
    V: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut map = f.debug_map();
        self.visit(|k, v| {
            map.entry(k, v);
            ControlFlow::<()>::Continue(())
        });
        map.finish()
    }
}

// TODO(#515): ungate this impl
#[cfg(feature = "alloc")]
impl<K, V, const E: usize> Eq for ArchivedBTreeMap<K, V, E>
where
    K: PartialEq,
    V: PartialEq,
{
}

impl<K, V, Q, const E: usize> Index<&Q> for ArchivedBTreeMap<K, V, E>
where
    Q: Ord + ?Sized,
    K: Borrow<Q> + Ord,
{
    type Output = V;

    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).unwrap()
    }
}

// TODO(#515): ungate this impl
#[cfg(feature = "alloc")]
impl<K, V, const E1: usize, const E2: usize>
    PartialEq<ArchivedBTreeMap<K, V, E2>> for ArchivedBTreeMap<K, V, E1>
where
    K: PartialEq,
    V: PartialEq,
{
    fn eq(&self, other: &ArchivedBTreeMap<K, V, E2>) -> bool {
        if self.len() != other.len() {
            return false;
        }
        let mut i = other.iter();
        self.visit(|lk, lv| {
            let (rk, rv) = i.next().unwrap();
            if lk != rk || lv != rv {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
        .is_none()
    }
}

impl<K: core::hash::Hash, V: core::hash::Hash, const E: usize> core::hash::Hash
    for ArchivedBTreeMap<K, V, E>
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.visit(|k, v| {
            (*k).hash(state);
            (*v).hash(state);
            ControlFlow::<()>::Continue(())
        });
    }
}

/// The resolver for [`ArchivedBTreeMap`].
pub struct BTreeMapResolver {
    root_node_pos: FixedUsize,
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::{alloc::Layout, error::Error, fmt, ptr::addr_of};

    use bytecheck::{CheckBytes, Verify};
    use rancor::{fail, Fallible, Source};

    use super::{ArchivedBTreeMap, InnerNode, Node};
    use crate::{
        collections::btree_map::{LeafNode, NodeKind},
        validation::{ArchiveContext, ArchiveContextExt as _},
        RelPtr,
    };

    #[derive(Debug)]
    struct InvalidLength {
        len: usize,
        maximum: usize,
    }

    impl fmt::Display for InvalidLength {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Invalid length in B-tree node: len {} was greater than \
                 maximum {}",
                self.len, self.maximum
            )
        }
    }

    impl Error for InvalidLength {}

    unsafe impl<C, K, V, const E: usize> Verify<C> for ArchivedBTreeMap<K, V, E>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let len = self.len();

            if len == 0 {
                return Ok(());
            }

            check_node_rel_ptr::<C, K, V, E>(&self.root, context)
        }
    }

    fn check_node_rel_ptr<C, K, V, const E: usize>(
        node_rel_ptr: &RelPtr<Node<K, V, E>>,
        context: &mut C,
    ) -> Result<(), C::Error>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        let node_ptr = node_rel_ptr.as_ptr_wrapping().cast::<Node<K, V, E>>();
        context.check_subtree_ptr(
            node_ptr.cast::<u8>(),
            &Layout::new::<Node<K, V, E>>(),
        )?;

        // SAFETY: We checked to make sure that `node_ptr` is properly aligned
        // and dereferenceable by calling `check_subtree_ptr`.
        let kind_ptr = unsafe { addr_of!((*node_ptr).kind) };
        // SAFETY: `kind_ptr` is a pointer to a subfield of `node_ptr` and so is
        // also properly aligned and dereferenceable.
        unsafe {
            CheckBytes::check_bytes(kind_ptr, context)?;
        }
        // SAFETY: `kind_ptr` was always properly aligned and dereferenceable,
        // and we just checked to make sure it pointed to a valid `NodeKind`.
        let kind = unsafe { kind_ptr.read() };

        match kind {
            NodeKind::Leaf => {
                // SAFETY:
                // We checked to make sure that `node_ptr` is properly aligned,
                // dereferenceable, and contained entirely within `context`'s
                // buffer by calling `check_subtree_ptr`.
                unsafe {
                    check_leaf_node::<C, K, V, E>(node_ptr.cast(), context)?
                }
            }
            NodeKind::Inner => {
                // SAFETY:
                // We checked to make sure that `node_ptr` is properly aligned
                // and dereferenceable.
                unsafe {
                    check_inner_node::<C, K, V, E>(node_ptr.cast(), context)?
                }
            }
        }

        Ok(())
    }

    /// # Safety
    ///
    /// `node_ptr` must be properly aligned, dereferenceable, and contained
    /// within `context`'s buffer.
    unsafe fn check_leaf_node<C, K, V, const E: usize>(
        node_ptr: *const LeafNode<K, V, E>,
        context: &mut C,
    ) -> Result<(), C::Error>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        context.in_subtree(node_ptr, |context| {
            // SAFETY: We checked to make sure that `node_ptr` is properly
            // aligned and dereferenceable by calling
            // `check_subtree_ptr`.
            let len_ptr = unsafe { addr_of!((*node_ptr).len) };
            // SAFETY: `len_ptr` is a pointer to a subfield of `node_ptr` and so
            // is also properly aligned and dereferenceable.
            unsafe {
                CheckBytes::check_bytes(len_ptr, context)?;
            }
            // SAFETY: `len_ptr` was always properly aligned and
            // dereferenceable, and we just checked to make sure it
            // pointed to a valid `ArchivedUsize`.
            let len = unsafe { &*len_ptr };
            let len = len.to_native() as usize;
            if len > E {
                fail!(InvalidLength { len, maximum: E });
            }

            // SAFETY: We checked that `node_ptr` is properly-aligned and
            // dereferenceable.
            let node_ptr = unsafe { addr_of!((*node_ptr).node) };
            // SAFETY:
            // - We checked that `node_ptr` is properly aligned and
            //   dereferenceable.
            // - We checked that `len` is less than or equal to `E`.
            unsafe {
                check_node_entries(node_ptr, len, context)?;
            }

            Ok(())
        })
    }

    /// # Safety
    ///
    /// - `node_ptr` must point to a valid `Node<K, V, E>`.
    /// - `len` must be less than or equal to `E`.
    unsafe fn check_node_entries<C, K, V, const E: usize>(
        node_ptr: *const Node<K, V, E>,
        len: usize,
        context: &mut C,
    ) -> Result<(), C::Error>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        for i in 0..len {
            // SAFETY: The caller has guaranteed that `node_ptr` is properly
            // aligned and dereferenceable.
            let key_ptr = unsafe { addr_of!((*node_ptr).keys[i]).cast::<K>() };
            // SAFETY: The caller has guaranteed that `node_ptr` is properly
            // aligned and dereferenceable.
            let value_ptr =
                unsafe { addr_of!((*node_ptr).values[i]).cast::<V>() };
            unsafe {
                K::check_bytes(key_ptr, context)?;
            }
            // SAFETY: `value_ptr` is a subfield of a node, and so is guaranteed
            // to be properly aligned and point to enough bytes for a `V`.
            unsafe {
                V::check_bytes(value_ptr, context)?;
            }
        }

        Ok(())
    }

    /// # Safety
    ///
    /// - `node_ptr` must be properly aligned and dereferenceable.
    /// - `len` must be less than or equal to `E`.
    unsafe fn check_inner_node<C, K, V, const E: usize>(
        node_ptr: *const InnerNode<K, V, E>,
        context: &mut C,
    ) -> Result<(), C::Error>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        context.in_subtree(node_ptr, |context| {
            for i in 0..E {
                // SAFETY: `in_subtree` guarantees that `node_ptr` is properly
                // aligned and dereferenceable.
                let lesser_node_ptr =
                    unsafe { addr_of!((*node_ptr).lesser_nodes[i]) };
                // SAFETY: `lesser_node_ptr` is a subfield of an inner node, and
                // so is guaranteed to be properly aligned and point to enough
                // bytes for a `RelPtr`.
                unsafe {
                    RelPtr::check_bytes(lesser_node_ptr, context)?;
                }
                // SAFETY: We just checked the `lesser_node_ptr` and it
                // succeeded, so it's safe to dereference.
                let lesser_node = unsafe { &*lesser_node_ptr };
                if !lesser_node.is_invalid() {
                    check_node_rel_ptr::<C, K, V, E>(lesser_node, context)?;
                }
            }
            // SAFETY: We checked that `node_ptr` is properly aligned and
            // dereferenceable.
            let greater_node_ptr =
                unsafe { addr_of!((*node_ptr).greater_node) };
            // SAFETY: `greater_node_ptr` is a subfield of an inner node, and so
            // is guaranteed to be properly aligned and point to enough bytes
            // for a `RelPtr`.
            unsafe {
                RelPtr::check_bytes(greater_node_ptr, context)?;
            }
            // SAFETY: We just checked the `greater_node_ptr` and it succeeded,
            // so it's safe to dereference.
            let greater_node = unsafe { &*greater_node_ptr };
            if !greater_node.is_invalid() {
                check_node_rel_ptr::<C, K, V, E>(greater_node, context)?;
            }

            // SAFETY: We checked that `node_ptr` is properly aligned and
            // dereferenceable.
            let node_ptr = unsafe { addr_of!((*node_ptr).node) };
            // SAFETY:
            // - The caller has guaranteed that `node_ptr` points to a valid
            //   `Node<K, V, E>`.
            // - All inner nodes have `E` items, and `E` is less than or equal
            //   to `E`.
            unsafe {
                check_node_entries::<C, K, V, E>(node_ptr, E, context)?;
            }

            Ok(())
        })
    }
}

#[cfg(all(test, feature = "alloc"))]
mod tests {
    use core::hash::{Hash, Hasher};

    use ahash::AHasher;

    use crate::{
        alloc::{collections::BTreeMap, string::ToString},
        api::test::to_archived,
        primitive::ArchivedU32,
    };

    #[test]
    fn test_hash() {
        let mut map = BTreeMap::new();
        map.insert("a".to_string(), 1);
        map.insert("b".to_string(), 2);

        to_archived(&map, |archived_map| {
            let mut hasher = AHasher::default();
            archived_map.hash(&mut hasher);
            let hash_value = hasher.finish();

            let mut expected_hasher = AHasher::default();
            for (k, v) in &map {
                k.hash(&mut expected_hasher);
                v.hash(&mut expected_hasher);
            }
            let expected_hash_value = expected_hasher.finish();

            assert_eq!(hash_value, expected_hash_value);
        });
    }

    #[test]
    fn test_range_empty() {
        let map = BTreeMap::<char, char>::new();
        to_archived(&map, |archived_map| {
            for _ in archived_map.range(..) {
                panic!("ArchivedBTreeMap should be empty");
            }
        });
    }

    #[test]
    fn test_range_one() {
        let mut map = BTreeMap::<i32, i32>::new();
        map.insert(1, 1);
        to_archived(&map, |archived_map| {
            for _ in archived_map.range_with(2.., |q, k| q.cmp(&k.to_native()))
            {
                panic!("ArchivedBTreeMap range should be empty");
            }
        })
    }

    #[test]
    fn test_range_open() {
        let mut map = BTreeMap::new();
        for i in 'a'..'z' {
            map.insert(i, i);
        }

        to_archived(&map, |archived_map| {
            for _ in
                archived_map.range_with(..'a', |q, k| q.cmp(&k.to_native()))
            {
                panic!("Range should be empty");
            }
            for _ in
                archived_map.range_with('|'.., |q, k| q.cmp(&k.to_native()))
            {
                panic!("Range should be empty");
            }
        });
    }

    #[test]
    fn test_range_str() {
        let mut map = BTreeMap::new();
        for i in 'a'..'z' {
            map.insert(i.to_string(), i.to_string());
        }

        to_archived(&map, |archived_map| {
            let start = 'd';
            let end = 'w';

            for ((k, v), expected) in archived_map
                .range_with(start..end, |q, k| {
                    q.cmp(&k.chars().next().unwrap())
                })
                .zip(start..end)
            {
                let expected = expected.to_string();
                assert_eq!(k.as_str(), expected);
                assert_eq!(v.as_str(), expected);
            }
        });
    }

    #[test]
    fn test_range_u32() {
        let mut map = BTreeMap::new();
        for i in 0..200 {
            map.insert(i as u32, i as u32);
        }

        to_archived(&map, |archived_map| {
            const START: u32 = 32;
            const END: u32 = 100;
            let start = ArchivedU32::from_native(START);
            let end = ArchivedU32::from_native(END);

            for ((k, v), expected) in
                archived_map.range(start..end).zip(START..END)
            {
                assert_eq!(k.to_native(), expected);
                assert_eq!(v.to_native(), expected);
            }
        });
    }
}
