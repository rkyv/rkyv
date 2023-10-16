use crate::{
    ops::{
        ArchivedRange, ArchivedRangeFrom, ArchivedRangeInclusive,
        ArchivedRangeTo, ArchivedRangeToInclusive,
    },
    Archive, Archived, Deserialize, Serialize,
};
use core::ops::{
    Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive,
};

// RangeFull

impl Archive for RangeFull {
    type Archived = Self;
    type Resolver = ();

    #[inline]
    unsafe fn resolve(
        &self,
        _: usize,
        _: Self::Resolver,
        _: *mut Self::Archived,
    ) {
    }
}

impl<S: ?Sized, E> Serialize<S, E> for RangeFull {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, E> {
        Ok(())
    }
}

impl<D: ?Sized, E> Deserialize<RangeFull, D, E> for RangeFull {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<Self, E> {
        Ok(RangeFull)
    }
}

// Range

impl<T: Archive> Archive for Range<T> {
    type Archived = ArchivedRange<T::Archived>;
    type Resolver = Range<T::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.start);
        self.start.resolve(pos + fp, resolver.start, fo);
        let (fp, fo) = out_field!(out.end);
        self.end.resolve(pos + fp, resolver.end, fo);
    }
}

impl<T: Serialize<S, E>, S: ?Sized, E> Serialize<S, E> for Range<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(Range {
            start: self.start.serialize(serializer)?,
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T: Archive, D: ?Sized, E> Deserialize<Range<T>, D, E>
    for Archived<Range<T>>
where
    T::Archived: Deserialize<T, D, E>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Range<T>, E> {
        Ok(Range {
            start: self.start.deserialize(deserializer)?,
            end: self.end.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<Range<T>> for ArchivedRange<U> {
    #[inline]
    fn eq(&self, other: &Range<T>) -> bool {
        self.start.eq(&other.start) && self.end.eq(&other.end)
    }
}

// RangeInclusive

impl<T: Archive> Archive for RangeInclusive<T> {
    type Archived = ArchivedRangeInclusive<T::Archived>;
    type Resolver = Range<T::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.start);
        self.start().resolve(pos + fp, resolver.start, fo);
        let (fp, fo) = out_field!(out.end);
        self.end().resolve(pos + fp, resolver.end, fo);
    }
}

impl<T: Serialize<S, E>, S: ?Sized, E> Serialize<S, E> for RangeInclusive<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(Range {
            start: self.start().serialize(serializer)?,
            end: self.end().serialize(serializer)?,
        })
    }
}

impl<T, D, E> Deserialize<RangeInclusive<T>, D, E> for Archived<RangeInclusive<T>>
where
    T: Archive,
    T::Archived: Deserialize<T, D, E>,
    D: ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeInclusive<T>, E> {
        Ok(RangeInclusive::new(
            self.start.deserialize(deserializer)?,
            self.end.deserialize(deserializer)?,
        ))
    }
}

impl<T, U: PartialEq<T>> PartialEq<RangeInclusive<T>>
    for ArchivedRangeInclusive<U>
{
    #[inline]
    fn eq(&self, other: &RangeInclusive<T>) -> bool {
        self.start.eq(other.start()) && self.end.eq(other.end())
    }
}

// RangeFrom

impl<T: Archive> Archive for RangeFrom<T> {
    type Archived = ArchivedRangeFrom<T::Archived>;
    type Resolver = RangeFrom<T::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.start);
        self.start.resolve(pos + fp, resolver.start, fo);
    }
}

impl<T: Serialize<S, E>, S: ?Sized, E> Serialize<S, E> for RangeFrom<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(RangeFrom {
            start: self.start.serialize(serializer)?,
        })
    }
}

impl<T, D, E> Deserialize<RangeFrom<T>, D, E> for Archived<RangeFrom<T>>
where
    T: Archive,
    D: ?Sized,
    T::Archived: Deserialize<T, D, E>,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeFrom<T>, E> {
        Ok(RangeFrom {
            start: self.start.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<RangeFrom<T>> for ArchivedRangeFrom<U> {
    #[inline]
    fn eq(&self, other: &RangeFrom<T>) -> bool {
        self.start.eq(&other.start)
    }
}

// RangeTo

impl<T: Archive> Archive for RangeTo<T> {
    type Archived = ArchivedRangeTo<T::Archived>;
    type Resolver = RangeTo<T::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.end);
        self.end.resolve(pos + fp, resolver.end, fo);
    }
}

impl<T: Serialize<S, E>, S: ?Sized, E> Serialize<S, E> for RangeTo<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(RangeTo {
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T, D, E> Deserialize<RangeTo<T>, D, E> for Archived<RangeTo<T>>
where
    T: Archive,
    D: ?Sized,
    T::Archived: Deserialize<T, D, E>,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeTo<T>, E> {
        Ok(RangeTo {
            end: self.end.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<RangeTo<T>> for ArchivedRangeTo<U> {
    #[inline]
    fn eq(&self, other: &RangeTo<T>) -> bool {
        self.end.eq(&other.end)
    }
}

// RangeToInclusive

impl<T: Archive> Archive for RangeToInclusive<T> {
    type Archived = ArchivedRangeToInclusive<T::Archived>;
    type Resolver = RangeToInclusive<T::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let (fp, fo) = out_field!(out.end);
        self.end.resolve(pos + fp, resolver.end, fo);
    }
}

impl<T: Serialize<S, E>, S: ?Sized, E> Serialize<S, E> for RangeToInclusive<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        Ok(RangeToInclusive {
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T, D, E> Deserialize<RangeToInclusive<T>, D, E> for Archived<RangeToInclusive<T>>
where
    T: Archive,
    T::Archived: Deserialize<T, D, E>,
    D: ?Sized,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeToInclusive<T>, E> {
        Ok(RangeToInclusive {
            end: self.end.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<RangeToInclusive<T>>
    for ArchivedRangeToInclusive<U>
{
    #[inline]
    fn eq(&self, other: &RangeToInclusive<T>) -> bool {
        self.end.eq(&other.end)
    }
}
