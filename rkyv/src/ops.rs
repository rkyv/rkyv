//! Archived versions of `ops` types.

use core::{
    cmp, fmt,
    ops::{Bound, RangeBounds},
};

use crate::{seal::Seal, Portable};

/// An archived [`RangeFull`](::core::ops::RangeFull).
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedRangeFull;

impl fmt::Debug for ArchivedRangeFull {
    #[inline]
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "..")
    }
}

/// An archived [`Range`](::core::ops::Range).
#[derive(Clone, Default, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedRange<T> {
    /// The lower bound of the range (inclusive).
    pub start: T,
    /// The upper bound of the range (inclusive).
    pub end: T,
}

impl<T: fmt::Debug> fmt::Debug for ArchivedRange<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.start.fmt(fmt)?;
        write!(fmt, "..")?;
        self.end.fmt(fmt)?;
        Ok(())
    }
}

impl<T: PartialOrd<T>> ArchivedRange<T> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: PartialOrd<T> + ?Sized,
    {
        <Self as RangeBounds<T>>::contains(self, item)
    }

    /// Returns `true` if the range contains no items.
    pub fn is_empty(&self) -> bool {
        match self.start.partial_cmp(&self.end) {
            None
            | Some(cmp::Ordering::Greater)
            | Some(cmp::Ordering::Equal) => true,
            Some(cmp::Ordering::Less) => false,
        }
    }
}

impl<T> RangeBounds<T> for ArchivedRange<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(&self.end)
    }
}

/// An archived [`RangeInclusive`](::core::ops::RangeInclusive).
#[derive(Clone, Default, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedRangeInclusive<T> {
    /// The lower bound of the range (inclusive).
    pub start: T,
    /// The upper bound of the range (inclusive).
    pub end: T,
}

impl<T: fmt::Debug> fmt::Debug for ArchivedRangeInclusive<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.start.fmt(fmt)?;
        write!(fmt, "..=")?;
        self.end.fmt(fmt)?;
        Ok(())
    }
}

impl<T: PartialOrd<T>> ArchivedRangeInclusive<T> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: PartialOrd<T> + ?Sized,
    {
        <Self as RangeBounds<T>>::contains(self, item)
    }

    /// Returns `true` if the range contains no items.
    pub fn is_empty(&self) -> bool {
        match self.start.partial_cmp(&self.end) {
            None | Some(cmp::Ordering::Greater) => true,
            Some(cmp::Ordering::Less) | Some(cmp::Ordering::Equal) => false,
        }
    }
}

impl<T> RangeBounds<T> for ArchivedRangeInclusive<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

/// An archived [`RangeFrom`](::core::ops::RangeFrom).
#[derive(Clone, Default, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedRangeFrom<T> {
    /// The lower bound of the range (inclusive).
    pub start: T,
}

impl<T: fmt::Debug> fmt::Debug for ArchivedRangeFrom<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.start.fmt(fmt)?;
        write!(fmt, "..")?;
        Ok(())
    }
}

impl<T: PartialOrd<T>> ArchivedRangeFrom<T> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: ?Sized + PartialOrd<T>,
    {
        <Self as RangeBounds<T>>::contains(self, item)
    }
}

impl<T> RangeBounds<T> for ArchivedRangeFrom<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Included(&self.start)
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }
}

/// An archived [`RangeTo`](::core::ops::RangeTo).
#[derive(Clone, Default, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedRangeTo<T> {
    /// The upper bound of the range (exclusive).
    pub end: T,
}

impl<T: fmt::Debug> fmt::Debug for ArchivedRangeTo<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "..")?;
        self.end.fmt(fmt)?;
        Ok(())
    }
}

impl<T: PartialOrd<T>> ArchivedRangeTo<T> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: ?Sized + PartialOrd<T>,
    {
        <Self as RangeBounds<T>>::contains(self, item)
    }
}

impl<T> RangeBounds<T> for ArchivedRangeTo<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Excluded(&self.end)
    }
}

/// An archived [`RangeToInclusive`](::core::ops::RangeToInclusive).
#[derive(Clone, Default, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedRangeToInclusive<T> {
    /// The upper bound of the range (inclusive).
    pub end: T,
}

impl<T: fmt::Debug> fmt::Debug for ArchivedRangeToInclusive<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "..=")?;
        self.end.fmt(fmt)?;
        Ok(())
    }
}

impl<T: PartialOrd<T>> ArchivedRangeToInclusive<T> {
    /// Returns `true` if `item` is contained in the range.
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: ?Sized + PartialOrd<T>,
    {
        <Self as RangeBounds<T>>::contains(self, item)
    }
}

impl<T> RangeBounds<T> for ArchivedRangeToInclusive<T> {
    fn start_bound(&self) -> Bound<&T> {
        Bound::Unbounded
    }

    fn end_bound(&self) -> Bound<&T> {
        Bound::Included(&self.end)
    }
}

/// An archived [`Bound`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Portable)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(u8)]
#[rkyv(crate)]
pub enum ArchivedBound<T> {
    /// An inclusive bound.
    Included(T),
    /// An exclusive bound.
    Excluded(T),
    /// An infinite endpoint. Indicates that there is no bound in this
    /// direction.
    Unbounded,
}

impl<T> ArchivedBound<T> {
    /// Converts from `&ArchivedBound<T>` to `Bound<&T>`.
    pub fn as_ref(&self) -> Bound<&T> {
        match self {
            ArchivedBound::Included(x) => Bound::Included(x),
            ArchivedBound::Excluded(x) => Bound::Excluded(x),
            ArchivedBound::Unbounded => Bound::Unbounded,
        }
    }

    /// Converts from `&mut ArchivedBound<T>` to `Bound<&mut T>`.
    pub fn as_mut(&mut self) -> Bound<&mut T> {
        match self {
            ArchivedBound::Included(x) => Bound::Included(x),
            ArchivedBound::Excluded(x) => Bound::Excluded(x),
            ArchivedBound::Unbounded => Bound::Unbounded,
        }
    }

    /// Converts from `Seal<&ArchivedBound<T>>` to `Bound<Seal<&T>>`.
    pub fn as_seal(this: Seal<'_, Self>) -> Bound<Seal<'_, T>> {
        let this = unsafe { Seal::unseal_unchecked(this) };
        match this {
            ArchivedBound::Included(x) => Bound::Included(Seal::new(x)),
            ArchivedBound::Excluded(x) => Bound::Excluded(Seal::new(x)),
            ArchivedBound::Unbounded => Bound::Unbounded,
        }
    }
}
