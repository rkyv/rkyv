//! [`Archive`] implementations for ranges.

use crate::{offset_of, Archive, ArchiveCopy, Archived, CopyResolver, Resolve, Serialize, Deserialize, Write};
use core::{
    cmp, fmt,
    ops::{Bound, Range, RangeBounds, RangeFull, RangeInclusive},
};

impl Archive for RangeFull {
    type Archived = Self;
    type Resolver = CopyResolver;
}

impl<W: Write + ?Sized> Serialize<W> for RangeFull {
    fn serialize(&self, _: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(CopyResolver)
    }
}

unsafe impl ArchiveCopy for RangeFull {}

impl Deserialize<RangeFull> for RangeFull {
    fn deserialize(&self) -> Self {
        RangeFull
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

impl<T: Archive> Resolve<Range<T>> for Range<T::Resolver> {
    type Archived = ArchivedRange<T::Archived>;

    fn resolve(self, pos: usize, value: &Range<T>) -> Self::Archived {
        ArchivedRange {
            start: self
                .start
                .resolve(pos + offset_of!(Self::Archived, start), &value.start),
            end: self
                .end
                .resolve(pos + offset_of!(Self::Archived, end), &value.end),
        }
    }
}

impl<T: Archive> Archive for Range<T> {
    type Archived = ArchivedRange<T::Archived>;
    type Resolver = Range<T::Resolver>;
}

impl<T: Serialize<W>, W: Write + ?Sized> Serialize<W> for Range<T> {
    fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(Range {
            start: self.start.serialize(writer)?,
            end: self.end.serialize(writer)?,
        })
    }
}

impl<T: Archive> Deserialize<Range<T>> for Archived<Range<T>>
where
    T::Archived: Deserialize<T>,
{
    fn deserialize(&self) -> Range<T> {
        Range {
            start: self.start.deserialize(),
            end: self.end.deserialize(),
        }
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
    pub fn contains<U>(&self, item: &U) -> bool
    where
        T: PartialOrd<U>,
        U: PartialOrd<T> + ?Sized,
    {
        <Self as RangeBounds<T>>::contains(self, item)
    }

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

impl<T: Archive> Resolve<RangeInclusive<T>> for Range<T::Resolver> {
    type Archived = ArchivedRangeInclusive<T::Archived>;

    fn resolve(self, pos: usize, value: &RangeInclusive<T>) -> Self::Archived {
        ArchivedRangeInclusive {
            start: self
                .start
                .resolve(pos + offset_of!(Self::Archived, start), &value.start()),
            end: self
                .end
                .resolve(pos + offset_of!(Self::Archived, end), &value.end()),
        }
    }
}

impl<T: Archive> Archive for RangeInclusive<T> {
    type Archived = ArchivedRangeInclusive<T::Archived>;
    type Resolver = Range<T::Resolver>;
}

impl<T: Serialize<W>, W: Write + ?Sized> Serialize<W> for RangeInclusive<T> {
    fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(Range {
            start: self.start().serialize(writer)?,
            end: self.end().serialize(writer)?,
        })
    }
}

impl<T: Archive> Deserialize<RangeInclusive<T>> for Archived<RangeInclusive<T>>
where
    T::Archived: Deserialize<T>,
{
    fn deserialize(&self) -> RangeInclusive<T> {
        RangeInclusive::new(self.start.deserialize(), self.end.deserialize())
    }
}
