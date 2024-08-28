use core::{
    hint::unreachable_unchecked,
    ops::{
        Bound, Range, RangeFrom, RangeFull, RangeInclusive, RangeTo,
        RangeToInclusive,
    },
};

use munge::munge;
use rancor::Fallible;

use crate::{
    ops::{
        ArchivedBound, ArchivedRange, ArchivedRangeFrom, ArchivedRangeFull,
        ArchivedRangeInclusive, ArchivedRangeTo, ArchivedRangeToInclusive,
    },
    traits::{CopyOptimization, NoUndef},
    Archive, Deserialize, Place, Serialize,
};

// RangeFull

impl Archive for RangeFull {
    const COPY_OPTIMIZATION: CopyOptimization<Self> =
        unsafe { CopyOptimization::enable() };

    type Archived = ArchivedRangeFull;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, _: Place<Self::Archived>) {}
}

impl<S: Fallible + ?Sized> Serialize<S> for RangeFull {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<RangeFull, D> for ArchivedRangeFull {
    fn deserialize(&self, _: &mut D) -> Result<RangeFull, D::Error> {
        Ok(RangeFull)
    }
}

impl PartialEq<RangeFull> for ArchivedRangeFull {
    fn eq(&self, _: &RangeFull) -> bool {
        true
    }
}

// Range

impl<T: Archive> Archive for Range<T> {
    type Archived = ArchivedRange<T::Archived>;
    type Resolver = Range<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedRange { start, end } = out);
        self.start.resolve(resolver.start, start);
        self.end.resolve(resolver.end, end);
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Range<T> {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(Range {
            start: self.start.serialize(serializer)?,
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T, D> Deserialize<Range<T>, D> for ArchivedRange<T::Archived>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Range<T>, D::Error> {
        Ok(Range {
            start: self.start.deserialize(deserializer)?,
            end: self.end.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<Range<T>> for ArchivedRange<U> {
    fn eq(&self, other: &Range<T>) -> bool {
        self.start.eq(&other.start) && self.end.eq(&other.end)
    }
}

// RangeInclusive

impl<T: Archive> Archive for RangeInclusive<T> {
    type Archived = ArchivedRangeInclusive<T::Archived>;
    type Resolver = Range<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedRangeInclusive { start, end } = out);
        self.start().resolve(resolver.start, start);
        self.end().resolve(resolver.end, end);
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for RangeInclusive<T> {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(Range {
            start: self.start().serialize(serializer)?,
            end: self.end().serialize(serializer)?,
        })
    }
}

impl<T, D> Deserialize<RangeInclusive<T>, D>
    for ArchivedRangeInclusive<T::Archived>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeInclusive<T>, D::Error> {
        Ok(RangeInclusive::new(
            self.start.deserialize(deserializer)?,
            self.end.deserialize(deserializer)?,
        ))
    }
}

impl<T, U> PartialEq<RangeInclusive<T>> for ArchivedRangeInclusive<U>
where
    U: PartialEq<T>,
{
    fn eq(&self, other: &RangeInclusive<T>) -> bool {
        self.start.eq(other.start()) && self.end.eq(other.end())
    }
}

// RangeFrom

impl<T: Archive> Archive for RangeFrom<T> {
    type Archived = ArchivedRangeFrom<T::Archived>;
    type Resolver = RangeFrom<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedRangeFrom { start } = out);
        self.start.resolve(resolver.start, start);
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for RangeFrom<T> {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(RangeFrom {
            start: self.start.serialize(serializer)?,
        })
    }
}

impl<T, D> Deserialize<RangeFrom<T>, D> for ArchivedRangeFrom<T::Archived>
where
    T: Archive,
    D: Fallible + ?Sized,
    T::Archived: Deserialize<T, D>,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeFrom<T>, D::Error> {
        Ok(RangeFrom {
            start: self.start.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<RangeFrom<T>> for ArchivedRangeFrom<U> {
    fn eq(&self, other: &RangeFrom<T>) -> bool {
        self.start.eq(&other.start)
    }
}

// RangeTo

impl<T: Archive> Archive for RangeTo<T> {
    type Archived = ArchivedRangeTo<T::Archived>;
    type Resolver = RangeTo<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedRangeTo { end } = out);
        self.end.resolve(resolver.end, end);
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for RangeTo<T> {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(RangeTo {
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T, D> Deserialize<RangeTo<T>, D> for ArchivedRangeTo<T::Archived>
where
    T: Archive,
    D: Fallible + ?Sized,
    T::Archived: Deserialize<T, D>,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeTo<T>, D::Error> {
        Ok(RangeTo {
            end: self.end.deserialize(deserializer)?,
        })
    }
}

impl<T, U: PartialEq<T>> PartialEq<RangeTo<T>> for ArchivedRangeTo<U> {
    fn eq(&self, other: &RangeTo<T>) -> bool {
        self.end.eq(&other.end)
    }
}

// RangeToInclusive

impl<T: Archive> Archive for RangeToInclusive<T> {
    type Archived = ArchivedRangeToInclusive<T::Archived>;
    type Resolver = RangeToInclusive<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        munge!(let ArchivedRangeToInclusive { end } = out);
        self.end.resolve(resolver.end, end);
    }
}

impl<T, S> Serialize<S> for RangeToInclusive<T>
where
    T: Serialize<S>,
    S: Fallible + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(RangeToInclusive {
            end: self.end.serialize(serializer)?,
        })
    }
}

impl<T, D> Deserialize<RangeToInclusive<T>, D>
    for ArchivedRangeToInclusive<T::Archived>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<RangeToInclusive<T>, D::Error> {
        Ok(RangeToInclusive {
            end: self.end.deserialize(deserializer)?,
        })
    }
}

impl<T, U> PartialEq<RangeToInclusive<T>> for ArchivedRangeToInclusive<U>
where
    U: PartialEq<T>,
{
    fn eq(&self, other: &RangeToInclusive<T>) -> bool {
        self.end.eq(&other.end)
    }
}

// Bound

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedBoundTag {
    Included,
    Excluded,
    Unbounded,
}

// SAFETY: `ArchivedBoundTag` is `repr(u8)` and so always consists of a single
// well-defined byte.
unsafe impl NoUndef for ArchivedBoundTag {}

#[repr(C)]
struct ArchivedBoundVariantIncluded<T>(ArchivedBoundTag, T);

#[repr(C)]
struct ArchivedBoundVariantExcluded<T>(ArchivedBoundTag, T);

#[repr(C)]
struct ArchivedBoundVariantUnbounded(ArchivedBoundTag);

impl<T: Archive> Archive for Bound<T> {
    type Archived = ArchivedBound<T::Archived>;
    type Resolver = Bound<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        match resolver {
            Bound::Included(resolver) => {
                let out = unsafe {
                    out.cast_unchecked::<
                    ArchivedBoundVariantIncluded<T::Archived>
                >()
                };
                munge!(let ArchivedBoundVariantIncluded(tag, out_value) = out);
                tag.write(ArchivedBoundTag::Included);

                let value = if let Bound::Included(value) = self.as_ref() {
                    value
                } else {
                    unsafe {
                        unreachable_unchecked();
                    }
                };

                value.resolve(resolver, out_value);
            }
            Bound::Excluded(resolver) => {
                let out = unsafe {
                    out.cast_unchecked::<
                    ArchivedBoundVariantExcluded<T::Archived>
                >()
                };
                munge!(let ArchivedBoundVariantExcluded(tag, out_value) = out);
                tag.write(ArchivedBoundTag::Excluded);

                let value = if let Bound::Excluded(value) = self.as_ref() {
                    value
                } else {
                    unsafe {
                        unreachable_unchecked();
                    }
                };

                value.resolve(resolver, out_value);
            }
            Bound::Unbounded => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedBoundVariantUnbounded>()
                };
                munge!(let ArchivedBoundVariantUnbounded(tag) = out);
                tag.write(ArchivedBoundTag::Unbounded);
            }
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Bound<T> {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        match self.as_ref() {
            Bound::Included(x) => x.serialize(serializer).map(Bound::Included),
            Bound::Excluded(x) => x.serialize(serializer).map(Bound::Excluded),
            Bound::Unbounded => Ok(Bound::Unbounded),
        }
    }
}

impl<T, D> Deserialize<Bound<T>, D> for ArchivedBound<T::Archived>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<Bound<T>, <D as Fallible>::Error> {
        Ok(match self {
            ArchivedBound::Included(value) => {
                Bound::Included(value.deserialize(deserializer)?)
            }
            ArchivedBound::Excluded(value) => {
                Bound::Excluded(value.deserialize(deserializer)?)
            }
            ArchivedBound::Unbounded => Bound::Unbounded,
        })
    }
}

impl<T, U> PartialEq<Bound<T>> for ArchivedBound<U>
where
    U: PartialEq<T>,
{
    fn eq(&self, other: &Bound<T>) -> bool {
        match (self, other) {
            (ArchivedBound::Included(this), Bound::Included(other))
            | (ArchivedBound::Excluded(this), Bound::Excluded(other)) => {
                this.eq(other)
            }
            (ArchivedBound::Unbounded, Bound::Unbounded) => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::ops::Bound;

    use crate::api::test::roundtrip;

    #[test]
    fn roundtrip_ranges() {
        roundtrip(&..);
        roundtrip(&(0u8..100u8));
        roundtrip(&(0u8..=100u8));
        roundtrip(&(0u8..));
        roundtrip(&(..100u8));
        roundtrip(&(..=100u8));
    }

    #[test]
    fn roundtrip_bound() {
        roundtrip(&Bound::Included(100u8));
        roundtrip(&Bound::Excluded(100u8));
        roundtrip(&Bound::<u8>::Unbounded);
    }
}
