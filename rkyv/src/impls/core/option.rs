use core::hint::unreachable_unchecked;

use munge::munge;
use rancor::Fallible;

use crate::{option::ArchivedOption, Archive, Deserialize, Place, Serialize};

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedOptionTag {
    None,
    Some,
}

#[repr(C)]
struct ArchivedOptionVariantNone(ArchivedOptionTag);

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

impl<T: Archive> Archive for Option<T> {
    type Archived = ArchivedOption<T::Archived>;
    type Resolver = Option<T::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    ) {
        match resolver {
            None => {
                let out = out.cast_unchecked::<ArchivedOptionVariantNone>();
                munge!(let ArchivedOptionVariantNone(tag) = out);
                tag.write(ArchivedOptionTag::None);
            }
            Some(resolver) => {
                let out = out
                    .cast_unchecked::<ArchivedOptionVariantSome<T::Archived>>();
                munge!(let ArchivedOptionVariantSome(tag, value_out) = out);
                tag.write(ArchivedOptionTag::Some);

                let value = if let Some(value) = self.as_ref() {
                    value
                } else {
                    unreachable_unchecked();
                };

                value.resolve(resolver, value_out);
            }
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Option<T> {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        self.as_ref()
            .map(|value| value.serialize(serializer))
            .transpose()
    }
}

impl<T: Archive, D: Fallible + ?Sized> Deserialize<Option<T>, D>
    for ArchivedOption<T::Archived>
where
    T::Archived: Deserialize<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Option<T>, D::Error> {
        match self {
            ArchivedOption::Some(value) => {
                Ok(Some(value.deserialize(deserializer)?))
            }
            ArchivedOption::None => Ok(None),
        }
    }
}
