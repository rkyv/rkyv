//! Archived bitwise containers.

use core::{marker::PhantomData, ops::Deref};

use bitvec::{
    order::{BitOrder, Lsb0},
    slice::BitSlice,
    store::BitStore,
    view::{BitView, BitViewSized},
};

use crate::{primitive::ArchivedUsize, vec::ArchivedVec, Portable};

/// An archived `BitVec`.
// We also have to store the bit length in the archived `BitVec`.
// This is because when calling `as_raw_slice` we will get unwanted bits if the
// `BitVec` bit length is not a multiple of the bit size of T.
//
// TODO: verify that bit_len matches the archived vector len in a verify meta
#[derive(Portable)]
#[archive(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(C)]
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArchivedBitVec<T = ArchivedUsize, O = Lsb0> {
    pub(crate) inner: ArchivedVec<T>,
    pub(crate) bit_len: ArchivedUsize,
    pub(crate) _or: PhantomData<O>,
}

impl<T: BitStore, O: BitOrder> Deref for ArchivedBitVec<T, O> {
    type Target = BitSlice<T, O>;

    fn deref(&self) -> &Self::Target {
        &self.inner.view_bits::<O>()[..self.bit_len.to_native() as usize]
    }
}

/// An archived `BitArray`.
#[derive(Portable)]
#[archive(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedBitArray<A = [ArchivedUsize; 1], O = Lsb0> {
    pub(crate) inner: A,
    pub(crate) _or: PhantomData<O>,
}

impl<A: BitViewSized, O: BitOrder> ArchivedBitArray<A, O> {
    /// Gets the elements of the archived `BitArray` as a `BitSlice`.
    pub fn as_bitslice(&self) -> &BitSlice<A::Store, O> {
        self.deref()
    }
}

impl<A: BitViewSized, O: BitOrder> Deref for ArchivedBitArray<A, O> {
    type Target = BitSlice<A::Store, O>;

    fn deref(&self) -> &Self::Target {
        self.inner.view_bits::<O>()
    }
}
