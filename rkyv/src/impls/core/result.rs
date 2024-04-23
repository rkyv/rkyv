use core::hint::unreachable_unchecked;

use munge::munge;
use rancor::Fallible;

use crate::{
    place::Initialized, result::ArchivedResult, Archive, Deserialize, Place,
    Serialize,
};

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedResultTag {
    Ok,
    Err,
}

// SAFETY: `ArchivedResultTag` is `repr(u8)` and so is always initialized.
unsafe impl Initialized for ArchivedResultTag {}

#[repr(C)]
struct ArchivedResultVariantOk<T>(ArchivedResultTag, T);

#[repr(C)]
struct ArchivedResultVariantErr<U>(ArchivedResultTag, U);

impl<T: Archive, U: Archive> Archive for Result<T, U> {
    type Archived = ArchivedResult<T::Archived, U::Archived>;
    type Resolver = Result<T::Resolver, U::Resolver>;

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        match resolver {
            Ok(resolver) => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedResultVariantOk<T::Archived>>()
                };
                munge!(let ArchivedResultVariantOk(tag, value_out) = out);
                tag.write(ArchivedResultTag::Ok);

                match self.as_ref() {
                    Ok(value) => value.resolve(resolver, value_out),
                    Err(_) => unsafe { unreachable_unchecked() },
                }
            }
            Err(resolver) => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedResultVariantErr<U::Archived>>(
                    )
                };
                munge!(let ArchivedResultVariantErr(tag, err_out) = out);
                tag.write(ArchivedResultTag::Err);

                match self.as_ref() {
                    Ok(_) => unsafe { unreachable_unchecked() },
                    Err(err) => err.resolve(resolver, err_out),
                }
            }
        }
    }
}

impl<T: Serialize<S>, U: Serialize<S>, S: Fallible + ?Sized> Serialize<S>
    for Result<T, U>
{
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        Ok(match self.as_ref() {
            Ok(value) => Ok(value.serialize(serializer)?),
            Err(value) => Err(value.serialize(serializer)?),
        })
    }
}

impl<T, U, D> Deserialize<Result<T, U>, D>
    for ArchivedResult<T::Archived, U::Archived>
where
    T: Archive,
    U: Archive,
    D: Fallible + ?Sized,
    T::Archived: Deserialize<T, D>,
    U::Archived: Deserialize<U, D>,
{
    #[inline]
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<Result<T, U>, D::Error> {
        match self {
            ArchivedResult::Ok(value) => {
                Ok(Ok(value.deserialize(deserializer)?))
            }
            ArchivedResult::Err(err) => Ok(Err(err.deserialize(deserializer)?)),
        }
    }
}
