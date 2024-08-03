#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
use core::{marker::PhantomData, pin::Pin, ptr::addr_of_mut};

use crate::{
    collections::btree_map::{
        entries_to_height, ArchivedBTreeMap, InnerNode, LeafNode, Node,
        NodeKind,
    },
    RawRelPtr,
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
    pub fn iter_pin(self: Pin<&mut Self>) -> IterPin<'_, K, V, E> {
        let this = unsafe { Pin::into_inner_unchecked(self) as *mut Self };
        IterPin {
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
    pub fn values_pin(self: Pin<&mut Self>) -> ValuesPin<'_, K, V, E> {
        let this = unsafe { Pin::into_inner_unchecked(self) as *mut Self };
        ValuesPin {
            inner: unsafe { RawIter::new(this) },
            _phantom: PhantomData,
        }
    }
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
}

/// An iterator over the entires of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`iter_pin`](ArchivedBTreeMap::iter_pin)
/// method on [`ArchivedBTreeMap`]. See its documentation for more.
pub struct IterPin<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<Pin<&'a mut ArchivedBTreeMap<K, V, E>>>,
}

impl<'a, K, V, const E: usize> Iterator for IterPin<'a, K, V, E> {
    type Item = (&'a K, Pin<&'a mut V>);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(k, v)| {
            (unsafe { &*k }, unsafe { Pin::new_unchecked(&mut *v) })
        })
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
}

/// A mutable iterator over the values of an `ArchivedBTreeMap`.
///
/// This struct is created by the [`values_pin`](ArchivedBTreeMap::keys) method
/// on [`ArchivedBTreeMap`]. See its documentation for more.
pub struct ValuesPin<'a, K, V, const E: usize> {
    inner: RawIter<K, V, E>,
    _phantom: PhantomData<Pin<&'a mut ArchivedBTreeMap<K, V, E>>>,
}

impl<'a, K, V, const E: usize> Iterator for ValuesPin<'a, K, V, E> {
    type Item = Pin<&'a mut V>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|(_, v)| unsafe { Pin::new_unchecked(&mut *v) })
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
            let mut current = unsafe {
                RawRelPtr::as_ptr_raw(addr_of_mut!((*map).root))
                    .cast::<Node<K, V, E>>()
            };
            loop {
                stack.push((current, 0));
                let kind = unsafe { (*current).kind };
                match kind {
                    NodeKind::Inner => {
                        let inner = current.cast::<InnerNode<K, V, E>>();
                        let lesser =
                            unsafe { addr_of_mut!((*inner).lesser_nodes) };
                        current = unsafe {
                            RawRelPtr::as_ptr_raw(lesser.cast()).cast()
                        };
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

        let k = unsafe { addr_of_mut!((*current).keys).cast::<K>().add(i) };
        let v = unsafe { addr_of_mut!((*current).values).cast::<V>().add(i) };
        let next_i = i + 1;

        // Advance to the next item
        let kind = unsafe { (*current).kind };
        match kind {
            NodeKind::Inner => {
                let inner = current.cast::<InnerNode<K, V, E>>();
                if next_i < E {
                    // More values in the current node
                    self.stack.push((current, next_i));

                    // Recurse to a lesser if valid
                    let next_lesser = unsafe {
                        addr_of_mut!((*inner).lesser_nodes)
                            .cast::<RawRelPtr>()
                            .add(next_i)
                    };
                    let next_lesser_is_invalid =
                        unsafe { RawRelPtr::is_invalid_raw(next_lesser) };
                    if !next_lesser_is_invalid {
                        self.stack.push((
                            unsafe {
                                RawRelPtr::as_ptr_raw(next_lesser).cast()
                            },
                            0,
                        ));
                    }
                } else {
                    // Recurse to a greater if valid
                    let next_greater =
                        unsafe { addr_of_mut!((*inner).greater_node) };
                    let next_greater_is_invalid =
                        unsafe { RawRelPtr::is_invalid_raw(next_greater) };
                    if !next_greater_is_invalid {
                        self.stack.push((
                            unsafe {
                                RawRelPtr::as_ptr_raw(next_greater).cast()
                            },
                            0,
                        ));
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
