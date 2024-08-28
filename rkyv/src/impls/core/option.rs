use core::hint::unreachable_unchecked;

use munge::munge;
use rancor::Fallible;

use crate::{
    option::ArchivedOption, traits::NoUndef, Archive, Deserialize, Place,
    Serialize,
};

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedOptionTag {
    None,
    Some,
}

// SAFETY: `ArchivedOptionTag` is `repr(u8)` and so always consists of a single
// well-defined byte.
unsafe impl NoUndef for ArchivedOptionTag {}

#[repr(C)]
struct ArchivedOptionVariantNone(ArchivedOptionTag);

#[repr(C)]
struct ArchivedOptionVariantSome<T>(ArchivedOptionTag, T);

impl<T: Archive> Archive for Option<T> {
    type Archived = ArchivedOption<T::Archived>;
    type Resolver = Option<T::Resolver>;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        match resolver {
            None => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedOptionVariantNone>()
                };
                munge!(let ArchivedOptionVariantNone(tag) = out);
                tag.write(ArchivedOptionTag::None);
            }
            Some(resolver) => {
                let out = unsafe {
                    out
                    .cast_unchecked::<ArchivedOptionVariantSome<T::Archived>>()
                };
                munge!(let ArchivedOptionVariantSome(tag, out_value) = out);
                tag.write(ArchivedOptionTag::Some);

                let value = if let Some(value) = self.as_ref() {
                    value
                } else {
                    unsafe {
                        unreachable_unchecked();
                    }
                };

                value.resolve(resolver, out_value);
            }
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Option<T> {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        self.as_ref()
            .map(|value| value.serialize(serializer))
            .transpose()
    }
}

impl<T, D> Deserialize<Option<T>, D> for ArchivedOption<T::Archived>
where
    T: Archive,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<Option<T>, D::Error> {
        Ok(match self {
            ArchivedOption::Some(value) => {
                Some(value.deserialize(deserializer)?)
            }
            ArchivedOption::None => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::api::test::roundtrip;

    #[test]
    fn roundtrip_option() {
        roundtrip(&Option::<()>::None);
        roundtrip(&Some(42));
    }
}
