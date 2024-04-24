//! [`Archive`](crate::Archive) implementation for B-tree maps.

use core::{
    borrow::Borrow,
    cmp::Ordering,
    fmt,
    marker::PhantomData,
    mem::{size_of, MaybeUninit},
    ops::ControlFlow,
    slice,
};

use munge::munge;
use rancor::{fail, Fallible, Source};

use crate::{
    collections::util::IteratorLengthMismatch,
    place::Initialized,
    primitive::{ArchivedUsize, FixedUsize},
    ser::{Allocator, Writer, WriterExt as _},
    util::{InlineVec, SerVec},
    Place, Portable, RawRelPtr, Serialize,
};

// B-trees are typically characterized as having a branching factor of B.
// However, in this implementation our B-trees are characterized as having a
// number of entries per node E where E = B - 1. This is done because it's
// easier to add an additional node pointer to each inner node than it is to
// store one less entry per inner node. Because generic const exprs are not
// stable, we can't declare a field `entries: [Entry; { B - 1 }]`. But we can
// declare `branches: [RawRelPtr; N]` and then add another `last: RawRelPtr`
// field. When the branching factor B is needed, it will be calculated as E + 1.

#[inline]
const fn nodes_in_level<const E: usize>(i: u32) -> usize {
    // The root of the tree has one node, and each level down has B times as
    // many nodes at the last. Therefore, the number of nodes in the I-th level
    // is equal to B^I.

    (E + 1).pow(i)
}

#[inline]
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

#[inline]
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
    ((E + 1) * n).ilog(E + 1)
}

#[inline]
const fn ll_entries<const E: usize>(height: u32, n: usize) -> usize {
    // The number of entries not in the last level is equal to the number of
    // entries in a full B-tree of height H - 1. The number of entries in
    // the last level is thus the total number of entries minus the number
    // of entries not in the last level.
    n - entries_in_full_tree::<E>(height - 1)
}

#[derive(Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[archive(crate)]
#[repr(u8)]
enum NodeKind {
    Leaf,
    Inner,
}

// SAFETY: `NodeKind` is `repr(u8)` and so is always initialized.
unsafe impl Initialized for NodeKind {}

#[derive(Portable)]
#[archive(crate)]
#[repr(C)]
struct Node<K, V, const E: usize> {
    kind: NodeKind,
    len: ArchivedUsize,
    keys: [MaybeUninit<K>; E],
    values: [MaybeUninit<V>; E],
}

#[derive(Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[archive(crate)]
#[repr(C)]
struct InnerNode<K, V, const E: usize> {
    node: Node<K, V, E>,
    lesser_nodes: [MaybeUninit<RawRelPtr>; E],
    greater_node: RawRelPtr,
}

/// An archived [`BTreeMap`](std::collections::BTreeMap).
#[derive(Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    check_bytes(verify)
)]
#[archive(crate)]
#[repr(C)]
pub struct ArchivedBTreeMap<K, V, const E: usize = 5> {
    // The type of the root node is determined at runtime because it may point
    // to:
    // - Nothing if the length is zero
    // - A leaf node if there is only one node
    // - Or an inner node if there are multiple nodes
    root: RawRelPtr,
    len: ArchivedUsize,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V, const E: usize> ArchivedBTreeMap<K, V, E> {
    /// Returns whether the B-tree map contains the given key.
    #[inline]
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        self.get_key_value(key).is_some()
    }

    /// Returns the value associated with the given key, or `None` if the key is
    /// not present in the B-tree map.
    #[inline]
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Ord + ?Sized,
        K: Borrow<Q> + Ord,
    {
        self.get_key_value(key).map(|(_, value)| value)
    }

    /// Returns true if the B-tree map contains no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of entries in the B-tree map.
    #[inline]
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
        if self.is_empty() {
            return None;
        }

        let mut current = unsafe { self.root.as_ptr().cast::<Node<K, V, E>>() };
        'outer: loop {
            let node = unsafe { &*current };
            for i in 0..node.len.to_native() as usize {
                let k = unsafe { node.keys[i].assume_init_ref() };
                match key.cmp(k.borrow()) {
                    Ordering::Equal => {
                        let v = unsafe { node.values[i].assume_init_ref() };
                        return Some((k, v));
                    }
                    Ordering::Less => match node.kind {
                        NodeKind::Inner => {
                            let inner_node = unsafe {
                                &*current.cast::<InnerNode<K, V, E>>()
                            };
                            let lesser_node = unsafe {
                                inner_node.lesser_nodes[i].assume_init_ref()
                            };
                            if !lesser_node.is_invalid() {
                                current = unsafe {
                                    lesser_node.as_ptr().cast::<Node<K, V, E>>()
                                };
                                continue 'outer;
                            } else {
                                return None;
                            }
                        }
                        NodeKind::Leaf => return None,
                    },
                    Ordering::Greater => (),
                }
            }
            match node.kind {
                NodeKind::Inner => {
                    let inner_node =
                        unsafe { &*current.cast::<InnerNode<K, V, E>>() };
                    if !inner_node.greater_node.is_invalid() {
                        current = unsafe {
                            inner_node
                                .greater_node
                                .as_ptr()
                                .cast::<Node<K, V, E>>()
                        };
                    } else {
                        return None;
                    }
                }
                NodeKind::Leaf => return None,
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
            RawRelPtr::emplace_invalid(root);
        } else {
            RawRelPtr::emplace(resolver.root_node_pos, root);
        }

        out_len.write(ArchivedUsize::from_native(len as FixedUsize));
    }

    /// Serializes an `ArchivedBTreeMap` from the given iterator and serializer.
    pub fn serialize_from_ordered_iter<'a, I, UK, UV, S>(
        mut iter: I,
        serializer: &mut S,
    ) -> Result<BTreeMapResolver, S::Error>
    where
        I: ExactSizeIterator<Item = (&'a UK, &'a UV)>,
        UK: 'a + Serialize<S, Archived = K>,
        UV: 'a + Serialize<S, Archived = V>,
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
                    open_inners.push(InlineVec::<
                        (&'a UK, &'a UV, Option<usize>),
                        E,
                    >::new());
                }

                let mut open_leaf = InlineVec::<(&'a UK, &'a UV), E>::new();

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
                                break;
                            }
                        }

                        for _ in 0..popped {
                            open_inners.push(InlineVec::default());
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
                    root_node_pos: child_node_pos.unwrap(),
                })
            },
        )?
    }

    fn close_leaf<UK, UV, S>(
        items: &[(&UK, &UV)],
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        UK: Serialize<S, Archived = K>,
        UV: Serialize<S, Archived = V>,
        S: Writer + Fallible + ?Sized,
    {
        let mut resolvers = InlineVec::<(UK::Resolver, UV::Resolver), E>::new();
        for (key, value) in items {
            resolvers.push((
                key.serialize(serializer)?,
                value.serialize(serializer)?,
            ));
        }

        let pos = serializer.align_for::<Node<K, V, E>>()?;
        let mut node = MaybeUninit::<Node<K, V, E>>::zeroed();

        let node_place =
            unsafe { Place::new_unchecked(pos, node.as_mut_ptr()) };

        munge!(let Node { kind, len, keys, values } = node_place);
        kind.write(NodeKind::Leaf);
        len.write(ArchivedUsize::from_native(items.len() as FixedUsize));
        for (i, ((k, v), (kr, vr))) in
            items.iter().zip(resolvers.drain(..)).enumerate()
        {
            let out_key = unsafe { keys.index(i).cast_unchecked() };
            k.resolve(kr, out_key);
            let out_value = unsafe { values.index(i).cast_unchecked() };
            v.resolve(vr, out_value);
        }

        let bytes = unsafe {
            slice::from_raw_parts(
                node.as_ptr().cast::<u8>(),
                size_of::<Node<K, V, E>>(),
            )
        };
        serializer.write(bytes)?;

        Ok(pos)
    }

    fn close_inner<UK, UV, S>(
        items: &[(&UK, &UV, Option<usize>)],
        greater_node_pos: Option<usize>,
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        UK: Serialize<S, Archived = K>,
        UV: Serialize<S, Archived = V>,
        S: Writer + Fallible + ?Sized,
    {
        let mut resolvers = InlineVec::<(UK::Resolver, UV::Resolver), E>::new();
        for (key, value, _) in items {
            resolvers.push((
                key.serialize(serializer)?,
                value.serialize(serializer)?,
            ));
        }

        let pos = serializer.align_for::<InnerNode<K, V, E>>()?;
        let mut node = MaybeUninit::<InnerNode<K, V, E>>::zeroed();

        let node_place =
            unsafe { Place::new_unchecked(pos, node.as_mut_ptr()) };

        munge! {
            let InnerNode {
                node: Node {
                    kind,
                    len,
                    keys,
                    values,
                },
                lesser_nodes,
                greater_node,
            } = node_place;
        }

        kind.write(NodeKind::Inner);
        len.write(ArchivedUsize::from_native(items.len() as FixedUsize));
        for (i, ((k, v, l), (kr, vr))) in
            items.iter().zip(resolvers.drain(..)).enumerate()
        {
            let out_key = unsafe { keys.index(i).cast_unchecked() };
            k.resolve(kr, out_key);
            let out_value = unsafe { values.index(i).cast_unchecked() };
            v.resolve(vr, out_value);

            let out_lesser_node =
                unsafe { lesser_nodes.index(i).cast_unchecked() };
            if let Some(lesser_node) = l {
                RawRelPtr::emplace(*lesser_node, out_lesser_node);
            } else {
                RawRelPtr::emplace_invalid(out_lesser_node);
            }
        }

        if let Some(greater_node_pos) = greater_node_pos {
            RawRelPtr::emplace(greater_node_pos, greater_node);
        } else {
            RawRelPtr::emplace_invalid(greater_node);
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
            let root_ptr =
                unsafe { self.root.as_ptr().cast::<Node<K, V, E>>() };
            match Self::visit_inner(root_ptr, &mut f) {
                ControlFlow::Continue(()) => None,
                ControlFlow::Break(x) => Some(x),
            }
        }
    }

    fn visit_inner<T>(
        current: *const Node<K, V, E>,
        f: &mut impl FnMut(&K, &V) -> ControlFlow<T>,
    ) -> ControlFlow<T> {
        let node = unsafe { &*current };
        for i in 0..node.len.to_native() as usize {
            let key = unsafe { node.keys[i].assume_init_ref() };
            let value = unsafe { node.values[i].assume_init_ref() };
            match node.kind {
                NodeKind::Leaf => (),
                NodeKind::Inner => {
                    let inner =
                        unsafe { &*current.cast::<InnerNode<K, V, E>>() };
                    let lesser =
                        unsafe { inner.lesser_nodes[i].assume_init_ref() };
                    if !lesser.is_invalid() {
                        let lesser_ptr =
                            unsafe { lesser.as_ptr().cast::<Node<K, V, E>>() };
                        Self::visit_inner(lesser_ptr, f)?;
                    }
                }
            }
            f(key, value)?;
        }

        match node.kind {
            NodeKind::Leaf => (),
            NodeKind::Inner => {
                let inner = unsafe { &*current.cast::<InnerNode<K, V, E>>() };
                if !inner.greater_node.is_invalid() {
                    let greater_ptr = unsafe {
                        inner.greater_node.as_ptr().cast::<Node<K, V, E>>()
                    };
                    Self::visit_inner(greater_ptr, f)?;
                }
            }
        }

        ControlFlow::Continue(())
    }

    // TODO: add entries iterator if alloc feature is enabled
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

/// The resolver for [`ArchivedBTreeMap`].
pub struct BTreeMapResolver {
    root_node_pos: usize,
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::{alloc::Layout, ptr::addr_of};

    use bytecheck::{CheckBytes, Verify};
    use rancor::{Fallible, Source};

    use super::{ArchivedBTreeMap, InnerNode, Node};
    use crate::{
        collections::btree_map::NodeKind,
        validation::{ArchiveContext, ArchiveContextExt as _},
        RawRelPtr,
    };

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

            unsafe { check_node_rel_ptr::<C, K, V, E>(&self.root, context) }
        }
    }

    unsafe fn check_node_rel_ptr<C, K, V, const E: usize>(
        node_rel_ptr: &RawRelPtr,
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

        let kind_ptr = addr_of!((*node_ptr).kind);
        unsafe {
            CheckBytes::check_bytes(kind_ptr, context)?;
        }
        let kind = unsafe { kind_ptr.read() };

        let len_ptr = addr_of!((*node_ptr).len);
        unsafe {
            CheckBytes::check_bytes(len_ptr, context)?;
        }
        let len = unsafe { &*len_ptr };

        match kind {
            NodeKind::Leaf => check_leaf_node::<C, K, V, E>(
                node_ptr,
                len.to_native() as usize,
                context,
            )?,
            NodeKind::Inner => check_inner_node::<C, K, V, E>(
                node_ptr.cast(),
                len.to_native() as usize,
                context,
            )?,
        }

        Ok(())
    }

    unsafe fn check_leaf_node<C, K, V, const E: usize>(
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
        context.check_subtree_ptr(
            node_ptr.cast::<u8>(),
            &Layout::new::<Node<K, V, E>>(),
        )?;
        let range = unsafe { context.push_prefix_subtree(node_ptr)? };

        check_node_entries(node_ptr, len, context)?;

        unsafe {
            context.pop_subtree_range(range)?;
        }
        Ok(())
    }

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
            let key_ptr = addr_of!((*node_ptr).keys).cast::<K>().add(i);
            K::check_bytes(key_ptr, context)?;
            let value_ptr = addr_of!((*node_ptr).values).cast::<V>().add(i);
            V::check_bytes(value_ptr, context)?;
        }

        Ok(())
    }

    unsafe fn check_inner_node<C, K, V, const E: usize>(
        node_ptr: *const InnerNode<K, V, E>,
        len: usize,
        context: &mut C,
    ) -> Result<(), C::Error>
    where
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
        K: CheckBytes<C>,
        V: CheckBytes<C>,
    {
        let inner_node_ptr = node_ptr.cast::<InnerNode<K, V, E>>();
        context.check_subtree_ptr(
            inner_node_ptr.cast::<u8>(),
            &Layout::new::<InnerNode<K, V, E>>(),
        )?;
        let range = unsafe { context.push_prefix_subtree(inner_node_ptr)? };

        for i in 0..len {
            let lesser_node_ptr = addr_of!((*node_ptr).lesser_nodes)
                .cast::<RawRelPtr>()
                .add(i);
            RawRelPtr::check_bytes(lesser_node_ptr, context)?;
            let lesser_node = unsafe { &*lesser_node_ptr };
            if !lesser_node.is_invalid() {
                check_node_rel_ptr::<C, K, V, E>(lesser_node, context)?;
            }
        }
        let greater_node_ptr = addr_of!((*node_ptr).greater_node);
        RawRelPtr::check_bytes(greater_node_ptr, context)?;
        let greater_node = unsafe { &*greater_node_ptr };
        if !greater_node.is_invalid() {
            check_node_rel_ptr::<C, K, V, E>(greater_node, context)?;
        }

        let node_ptr = unsafe { addr_of!((*node_ptr).node) };
        unsafe {
            check_node_entries::<C, K, V, E>(node_ptr, len, context)?;
        }

        unsafe {
            context.pop_subtree_range(range)?;
        }
        Ok(())
    }
}
