//! [`Archive`] implementations for ranges.

use crate::{offset_of, Archive, ArchiveCopy, Archived, Deserialize, Fallible, Serialize};
use core::{
    cmp, fmt,
    ops::{Bound, Range, RangeBounds, RangeFull, RangeInclusive},
};

impl Archive for RangeFull {
    type Archived = Self;
    type Resolver = ();

    fn resolve(&self, _: usize, _: Self::Resolver) -> Self::Archived {
        RangeFull
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for RangeFull {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

unsafe impl ArchiveCopy for RangeFull {}

impl<D: Fallible + ?Sized> Deserialize<RangeFull, D> for RangeFull {
    fn deserialize(&self, _: &mut D) -> Result<Self, D::Error> {
        Ok(RangeFull)
    }
}

/// An archived [`Range`].
#[derive(Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "strict", repr(C))]
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
            None | Some(cmp::Ordering::Greater) | Some(cmp::Ordering::Equal) => true,
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

impl<T, U: PartialEq<T>> PartialEq<Range<T>> for ArchivedRange<U> {
    fn eq(&self, other: &Range<T>) -> bool {
        self.start.eq(&other.start) && self.end.eq(&other.end)
    }
}

impl<T: Archive> Archive for Range<T> {
    type Archived = ArchivedRange<T::Archived>;
    type Resolver = Range<T::Resolver>;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedRange {
            start: self
                .start
                .resolve(pos + offset_of!(Self::Archived, start), resolver.start),
            end: self
                .end
                .resolve(pos + offset_of!(Self::Archived, end), resolver.end),
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Range<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(Range {
            start: self.start.serialize(serializer)?,
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T: Archive, D: Fallible + ?Sized> Deserialize<Range<T>, D> for Archived<Range<T>>
where
    T::Archived: Deserialize<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Range<T>, D::Error> {
        Ok(Range {
            start: self.start.deserialize(deserializer)?,
            end: self.end.deserialize(deserializer)?,
        })
    }
}

/// An archived [`RangeInclusive`].
#[derive(Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "strict", repr(C))]
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

impl<T, U: PartialEq<T>> PartialEq<RangeInclusive<T>> for ArchivedRangeInclusive<U> {
    fn eq(&self, other: &RangeInclusive<T>) -> bool {
        self.start.eq(other.start()) && self.end.eq(other.end())
    }
}

impl<T: Archive> Archive for RangeInclusive<T> {
    type Archived = ArchivedRangeInclusive<T::Archived>;
    type Resolver = Range<T::Resolver>;

    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
        ArchivedRangeInclusive {
            start: self
                .start()
                .resolve(pos + offset_of!(Self::Archived, start), resolver.start),
            end: self
                .end()
                .resolve(pos + offset_of!(Self::Archived, end), resolver.end),
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for RangeInclusive<T> {
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(Range {
            start: self.start().serialize(serializer)?,
            end: self.end().serialize(serializer)?,
        })
    }
}

impl<T: Archive, D: Fallible + ?Sized> Deserialize<RangeInclusive<T>, D>
    for Archived<RangeInclusive<T>>
where
    T::Archived: Deserialize<T, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<RangeInclusive<T>, D::Error> {
        Ok(RangeInclusive::new(
            self.start.deserialize(deserializer)?,
            self.end.deserialize(deserializer)?,
        ))
    }
}
