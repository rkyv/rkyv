use core::{
    borrow::Borrow,
    cmp::Ordering,
    iter::FusedIterator,
    marker::PhantomData,
    ops::{Bound, RangeBounds},
    ptr::addr_of_mut,
};

use crate::{
    alloc::vec::Vec,
    collections::btree_map::{
        entries_to_height, ArchivedBTreeMap, InnerNode, LeafNode, Node,
        NodeKind,
    },
    seal::Seal,
    RelPtr,
};

impl<K, V, const E: usize> ArchivedBTreeMap<K, V, E> {
    /// Gets an iterator over the entries of the map, sorted by key.
    pub fn iter(&self) -> Iter<'_, K, V, E> {
        let this = (self as *const Self).cast_mut();
        Iter {
            inner: unsafe { RawIter::new(this) },
            _phantom: PhantomData,
        }
    }

    /// Gets a mutable iterator over the entires of the map, sorted by key.
    pub fn iter_seal(this: Seal<'_, Self>) -> IterSeal<'_, K, V, E> {
        let this = unsafe { Seal::unseal_unchecked(this) as *mut Self };
        IterSeal {
            inner: unsafe { RawIter::new(this) },
            _phantom: PhantomData,
        }
    }

    /// Gets an iterator over the sorted keys of the map.
    pub fn keys(&self) -> Keys<'_, K, V, E> {
        let this = (self as *const Self).cast_mut();
        Keys {
            inner: unsafe { RawIter::new(this) },
            _phantom: PhantomData,
        }
    }

    /// Gets an iterator over the values of the map.
    pub fn values(&self) -> Values<'_, K, V, E> {
        let this = (self as *const Self).cast_mut();
        Values {
            inner: unsafe { RawIter::new(this) },
            _phantom: PhantomData,
        }
    }

    /// Gets a mutable iterator over the values of the map.
    pub fn values_seal(this: Seal<'_, Self>) -> ValuesSeal<'_, K, V, E> {
        let this = unsafe { Seal::unseal_unchecked(this) as *mut Self };
        ValuesSeal {
            inner: unsafe { RawIter::new(this) },
            _phantom: PhantomData,
        }
    }

    /// Gets an iterator over a sub-range of entries, sorted by key.
    pub fn range<Q, R>(&self, range: R) -> Range<'_, K, V, E>
    where
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        K: Borrow<Q> + Ord,
    {
        self.range_with(range, |q, k| q.cmp(k.borrow()))
    }

    /// Gets an iterator over a sub-range of entries, sorted by key.
    ///
    /// This method uses the supplied comparison function to compare the range
    /// to elements.
    pub fn range_with<Q, R, C>(&self, range: R, cmp: C) -> Range<'_, K, V, E>
    where
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let this = (self as *const Self).cast_mut();
        Range {
            inner: unsafe { RawRangeIter::new(this, range, cmp) },
            _phantom: PhantomData,
        }
    }

    /// Gets a mutable iterator over a sub-range of entries, sorted by key.
    pub fn range_seal<Q, R>(
        this: Seal<'_, Self>,
        range: R,
    ) -> RangeSeal<'_, K, V, E>
    where
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        K: Borrow<Q> + Ord,
    {
        Self::range_seal_with(this, range, |q, k| q.cmp(k.borrow()))
    }

    /// Gets a mutable iterator over a sub-range of entries, sorted by key.
    ///
    /// This method uses the supplied comparison function to compare the range
    /// to elements.
    pub fn range_seal_with<Q, R, C>(
        this: Seal<'_, Self>,
        range: R,
        cmp: C,
    ) -> RangeSeal<'_, K, V, E>
    where
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let this = unsafe { Seal::unseal_unchecked(this) as *mut Self };
        RangeSeal {
            inner: unsafe { RawRangeIter::new(this, range, cmp) },
            _phantom: PhantomData,
        }
    }
}

macro_rules! impl_iter_traits {
    ($($iter_ty:ident),*) => {
        $(
            impl<'a, K, V, const E: usize> ExactSizeIterator
                for $iter_ty<'a, K, V, E>
            {
            }

            impl<'a, K, V, const E: usize> FusedIterator
                for $iter_ty<'a, K, V, E>
            {
            }
        )*
    };
}

/// An iterator over the entires of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`iter`](ArchivedBTreeMap::iter) method on
/// [`ArchivedBTreeMap`]. See its documentation for more.
pub struct Iter<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<&'a ArchivedBTreeMap<K, V, E>>,
}

impl<'a, K, V, const E: usize> Iterator for Iter<'a, K, V, E> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (unsafe { &*k }, unsafe { &*v }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An iterator over the entires of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`iter_seal`](ArchivedBTreeMap::iter_seal)
/// method on [`ArchivedBTreeMap`]. See its documentation for more.
pub struct IterSeal<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<Seal<'a, ArchivedBTreeMap<K, V, E>>>,
}

impl<'a, K, V, const E: usize> Iterator for IterSeal<'a, K, V, E> {
    type Item = (&'a K, Seal<'a, V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (unsafe { &*k }, Seal::new(unsafe { &mut *v })))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An iterator over the keys of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`keys`](ArchivedBTreeMap::keys) method on
/// [`ArchivedBTreeMap`]. See its documentation for more.
pub struct Keys<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<&'a ArchivedBTreeMap<K, V, E>>,
}

impl<'a, K, V, const E: usize> Iterator for Keys<'a, K, V, E> {
    type Item = &'a K;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, _)| unsafe { &*k })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// An iterator over the values of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`values`](ArchivedBTreeMap::keys) method on
/// [`ArchivedBTreeMap`]. See its documentation for more.
pub struct Values<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<&'a ArchivedBTreeMap<K, V, E>>,
}

impl<'a, K, V, const E: usize> Iterator for Values<'a, K, V, E> {
    type Item = &'a V;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(_, v)| unsafe { &*v })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

/// A mutable iterator over the values of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`values_pin`](ArchivedBTreeMap::keys) method
/// on [`ArchivedBTreeMap`]. See its documentation for more.
pub struct ValuesSeal<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<Seal<'a, ArchivedBTreeMap<K, V, E>>>,
}

impl<'a, K, V, const E: usize> Iterator for ValuesSeal<'a, K, V, E> {
    type Item = Seal<'a, V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(_, v)| Seal::new(unsafe { &mut *v }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

struct RawIter<K, V, const E: usize> {
    remaining: usize,
    stack: Vec<(*mut Node<K, V, E>, usize)>,
}

impl<K, V, const E: usize> RawIter<K, V, E> {
    unsafe fn new(map: *mut ArchivedBTreeMap<K, V, E>) -> Self {
        let remaining = unsafe { (*map).len.to_native() as usize };
        let mut stack = Vec::new();
        if remaining != 0 {
            stack.reserve(entries_to_height::<E>(remaining) as usize);
            let mut current =
                unsafe { RelPtr::as_ptr_raw(addr_of_mut!((*map).root)) };
            loop {
                stack.push((current, 0));
                let kind = unsafe { (*current).kind };
                match kind {
                    NodeKind::Inner => {
                        let inner = current.cast::<InnerNode<K, V, E>>();
                        let lesser =
                            unsafe { addr_of_mut!((*inner).lesser_nodes[0]) };
                        current = unsafe { RelPtr::as_ptr_raw(lesser) };
                    }
                    NodeKind::Leaf => break,
                }
            }
        }

        Self { remaining, stack }
    }
}

impl<K, V, const E: usize> Iterator for RawIter<K, V, E> {
    type Item = (*mut K, *mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let (current, i) = self.stack.pop()?;
        self.remaining -= 1;

        let k = unsafe { addr_of_mut!((*current).keys[i]).cast::<K>() };
        let v = unsafe { addr_of_mut!((*current).values[i]).cast::<V>() };
        let next_i = i + 1;

        // Advance to the next item
        let kind = unsafe { (*current).kind };
        match kind {
            NodeKind::Inner => {
                let inner = current.cast::<InnerNode<K, V, E>>();
                let next = if next_i < E {
                    // More values in the current node
                    self.stack.push((current, next_i));

                    // Next is a lesser node
                    unsafe { addr_of_mut!((*inner).lesser_nodes[next_i]) }
                } else {
                    // Next is a greater node
                    unsafe { addr_of_mut!((*inner).greater_node) }
                };

                let next_is_invalid = unsafe { RelPtr::is_invalid_raw(next) };
                if !next_is_invalid {
                    // Recurse left on next node
                    let mut current = unsafe { RelPtr::as_ptr_raw(next) };
                    loop {
                        self.stack.push((current, 0));
                        let kind = unsafe { (*current).kind };
                        match kind {
                            NodeKind::Inner => {
                                let inner =
                                    current.cast::<InnerNode<K, V, E>>();
                                let lesser = unsafe {
                                    addr_of_mut!((*inner).lesser_nodes[0])
                                };
                                current = unsafe { RelPtr::as_ptr_raw(lesser) };
                            }
                            NodeKind::Leaf => break,
                        }
                    }
                }
            }
            NodeKind::Leaf => {
                let leaf = current.cast::<LeafNode<K, V, E>>();
                let len = unsafe { (*leaf).len.to_native() as usize };
                if next_i < len {
                    self.stack.push((current, next_i));
                }
            }
        }

        Some((k, v))
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.remaining;
        (remaining, Some(remaining))
    }
}

impl<K, V, const E: usize> ExactSizeIterator for RawIter<K, V, E> {}

impl_iter_traits!(Iter, IterSeal, Keys, Values, ValuesSeal);

/// An iterator over a sub-range of entries of an `ArchivedBTreeMap`.
pub struct Range<'a, K, V, const E: usize> {
    inner: RawRangeIter<K, V, E>,
    _phantom: PhantomData<&'a ArchivedBTreeMap<K, V, E>>,
}

impl<'a, K, V, const E: usize> Iterator for Range<'a, K, V, E> {
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (unsafe { &*k }, unsafe { &*v }))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

/// A mutable iterator over a sub-range of entries of an `ArchivedBTreeMap`.
pub struct RangeSeal<'a, K, V, const E: usize> {
    inner: RawRangeIter<K, V, E>,
    _phantom: PhantomData<Seal<'a, ArchivedBTreeMap<K, V, E>>>,
}

impl<'a, K, V, const E: usize> Iterator for RangeSeal<'a, K, V, E> {
    type Item = (&'a K, Seal<'a, V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(k, v)| (unsafe { &*k }, Seal::new(unsafe { &mut *v })))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<'a, K, V, const E: usize> FusedIterator for Range<'a, K, V, E> {}
impl<'a, K, V, const E: usize> FusedIterator for RangeSeal<'a, K, V, E> {}

struct RawRangeIter<K, V, const E: usize> {
    stack: Vec<(*mut Node<K, V, E>, usize)>,
    end_key: Option<*mut K>,
}

impl<K, V, const E: usize> RawRangeIter<K, V, E> {
    unsafe fn new<Q, R, C>(
        map: *mut ArchivedBTreeMap<K, V, E>,
        range: R,
        cmp: C,
    ) -> Self
    where
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let len = unsafe { (*map).len.to_native() as usize };
        if len == 0 {
            return Self {
                stack: Vec::new(),
                end_key: None,
            };
        }

        let mut stack =
            Vec::with_capacity(entries_to_height::<E>(len) as usize);

        unsafe { Self::init_stack_for_lower(map, &range, &mut stack, &cmp) };

        let end_key = unsafe { Self::find_first_past_upper(map, &range, &cmp) };

        // If the end key and starting key are the same, then there are no
        // elements to iterate. Clear the stack.
        if let Some(ek) = end_key {
            if let Some((node, idx)) = stack.last() {
                let k =
                    unsafe { addr_of_mut!((*(*node)).keys[*idx]).cast::<K>() };
                if k == ek {
                    stack.clear();
                }
            }
        }

        Self { stack, end_key }
    }

    fn key_satisfies_lower<Q, C>(key: &K, lower: Bound<&Q>, cmp: &C) -> bool
    where
        Q: Ord + ?Sized,
        C: Fn(&Q, &K) -> Ordering,
    {
        match lower {
            Bound::Unbounded => true,
            Bound::Included(lb) => cmp(lb, key).is_le(),
            Bound::Excluded(lb) => cmp(lb, key).is_lt(),
        }
    }

    unsafe fn init_stack_for_lower<Q, R, C>(
        map: *mut ArchivedBTreeMap<K, V, E>,
        range: &R,
        stack: &mut Vec<(*mut Node<K, V, E>, usize)>,
        cmp: &C,
    ) where
        Q: Ord + ?Sized,
        R: RangeBounds<Q>,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let lower = range.start_bound();

        if matches!(lower, Bound::Unbounded) {
            let mut current =
                unsafe { RelPtr::as_ptr_raw(addr_of_mut!((*map).root)) };
            loop {
                stack.push((current, 0));
                let kind = unsafe { (*current).kind };
                match kind {
                    NodeKind::Inner => {
                        let inner = current.cast::<InnerNode<K, V, E>>();
                        let lesser =
                            unsafe { addr_of_mut!((*inner).lesser_nodes[0]) };
                        if unsafe { RelPtr::is_invalid_raw(lesser) } {
                            break;
                        }
                        current = unsafe { RelPtr::as_ptr_raw(lesser) };
                    }
                    NodeKind::Leaf => break,
                }
            }
            return;
        }

        let mut current =
            unsafe { RelPtr::as_ptr_raw(addr_of_mut!((*map).root)) };
        'descend: loop {
            match unsafe { (*current).kind } {
                NodeKind::Inner => {
                    for i in 0..E {
                        let k_ptr = unsafe {
                            addr_of_mut!((*current).keys[i]).cast::<K>()
                        };
                        let k_ref = unsafe { &*k_ptr };
                        if Self::key_satisfies_lower(k_ref, lower, cmp) {
                            stack.push((current, i));
                            let inner = current.cast::<InnerNode<K, V, E>>();
                            let lesser = unsafe {
                                addr_of_mut!((*inner).lesser_nodes[i])
                            };
                            if unsafe { RelPtr::is_invalid_raw(lesser) } {
                                break 'descend;
                            } else {
                                current = unsafe { RelPtr::as_ptr_raw(lesser) };
                                continue 'descend;
                            }
                        }
                    }
                    let inner = current.cast::<InnerNode<K, V, E>>();
                    let greater =
                        unsafe { addr_of_mut!((*inner).greater_node) };
                    if unsafe { RelPtr::is_invalid_raw(greater) } {
                        break;
                    } else {
                        current = unsafe { RelPtr::as_ptr_raw(greater) };
                    }
                }
                NodeKind::Leaf => {
                    let leaf = current.cast::<LeafNode<K, V, E>>();
                    let len = unsafe { (*leaf).len.to_native() as usize };
                    for i in 0..len {
                        let k_ptr = unsafe {
                            addr_of_mut!((*current).keys[i]).cast::<K>()
                        };
                        let k_ref = unsafe { &*k_ptr };
                        if Self::key_satisfies_lower(k_ref, lower, cmp) {
                            stack.push((current, i));
                            break 'descend;
                        }
                    }
                    break;
                }
            }
        }
    }

    fn key_is_past_upper<Q, C>(key: &K, upper: Bound<&Q>, cmp: &C) -> bool
    where
        Q: Ord + ?Sized,
        C: Fn(&Q, &K) -> Ordering,
    {
        match upper {
            Bound::Unbounded => false,
            Bound::Included(ub) => cmp(ub, key).is_lt(),
            Bound::Excluded(ub) => cmp(ub, key).is_le(),
        }
    }

    unsafe fn find_first_past_upper<Q, R, C>(
        map: *mut ArchivedBTreeMap<K, V, E>,
        range: &R,
        cmp: &C,
    ) -> Option<*mut K>
    where
        Q: Ord + ?Sized,
        R: RangeBounds<Q> + ?Sized,
        C: Fn(&Q, &K) -> Ordering,
        K: Ord,
    {
        let upper = range.end_bound();

        match upper {
            Bound::Unbounded => None,
            Bound::Included(_) | Bound::Excluded(_) => {
                let mut current =
                    unsafe { RelPtr::as_ptr_raw(addr_of_mut!((*map).root)) };
                let mut candidate = None;
                'search: loop {
                    match unsafe { (*current).kind } {
                        NodeKind::Inner => {
                            for i in 0..E {
                                let k_ptr = unsafe {
                                    addr_of_mut!((*current).keys[i]).cast::<K>()
                                };
                                let k_ref = unsafe { &*k_ptr };
                                if Self::key_is_past_upper(k_ref, upper, cmp) {
                                    candidate = Some(k_ptr);
                                    let inner =
                                        current.cast::<InnerNode<K, V, E>>();
                                    let lesser = unsafe {
                                        addr_of_mut!((*inner).lesser_nodes[i])
                                    };
                                    if unsafe { RelPtr::is_invalid_raw(lesser) }
                                    {
                                        break 'search;
                                    } else {
                                        current = unsafe {
                                            RelPtr::as_ptr_raw(lesser)
                                        };
                                        continue 'search;
                                    }
                                }
                            }
                            let inner = current.cast::<InnerNode<K, V, E>>();
                            let greater =
                                unsafe { addr_of_mut!((*inner).greater_node) };
                            if unsafe { RelPtr::is_invalid_raw(greater) } {
                                break;
                            } else {
                                current =
                                    unsafe { RelPtr::as_ptr_raw(greater) };
                            }
                        }
                        NodeKind::Leaf => {
                            let leaf = current.cast::<LeafNode<K, V, E>>();
                            let len =
                                unsafe { (*leaf).len.to_native() as usize };
                            for i in 0..len {
                                let k_ptr = unsafe {
                                    addr_of_mut!((*current).keys[i]).cast::<K>()
                                };
                                let k_ref = unsafe { &*k_ptr };
                                if Self::key_is_past_upper(k_ref, upper, cmp) {
                                    return Some(k_ptr);
                                }
                            }
                            break;
                        }
                    }
                }
                candidate
            }
        }
    }
}

impl<K, V, const E: usize> Iterator for RawRangeIter<K, V, E> {
    type Item = (*mut K, *mut V);

    fn next(&mut self) -> Option<Self::Item> {
        let (current, i) = self.stack.pop()?;

        let k = unsafe { addr_of_mut!((*current).keys[i]).cast::<K>() };
        if let Some(end) = self.end_key {
            if k == end {
                self.stack.clear();
                return None;
            }
        }

        let v = unsafe { addr_of_mut!((*current).values[i]).cast::<V>() };
        let next_i = i + 1;

        match unsafe { (*current).kind } {
            NodeKind::Inner => {
                let inner = current.cast::<InnerNode<K, V, E>>();
                let next = if next_i < E {
                    self.stack.push((current, next_i));
                    unsafe { addr_of_mut!((*inner).lesser_nodes[next_i]) }
                } else {
                    unsafe { addr_of_mut!((*inner).greater_node) }
                };

                if !unsafe { RelPtr::is_invalid_raw(next) } {
                    let mut current = unsafe { RelPtr::as_ptr_raw(next) };
                    loop {
                        self.stack.push((current, 0));
                        match unsafe { (*current).kind } {
                            NodeKind::Inner => {
                                let inner =
                                    current.cast::<InnerNode<K, V, E>>();
                                let lesser = unsafe {
                                    addr_of_mut!((*inner).lesser_nodes[0])
                                };
                                current = unsafe { RelPtr::as_ptr_raw(lesser) };
                            }
                            NodeKind::Leaf => break,
                        }
                    }
                }
            }
            NodeKind::Leaf => {
                let leaf = current.cast::<LeafNode<K, V, E>>();
                let len = unsafe { (*leaf).len.to_native() as usize };
                if next_i < len {
                    self.stack.push((current, next_i));
                }
            }
        }

        Some((k, v))
    }
}
