use crate::{
    result::ArchivedResult, Archive, Deserialize, Serialize,
};
use core::{hint::unreachable_unchecked, ptr};
use rancor::Fallible;

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedResultTag {
    Ok,
    Err,
}

#[repr(C)]
struct ArchivedResultVariantOk<T>(ArchivedResultTag, T);

#[repr(C)]
struct ArchivedResultVariantErr<U>(ArchivedResultTag, U);

impl<T: Archive, U: Archive> Archive for Result<T, U> {
    type Archived = ArchivedResult<T::Archived, U::Archived>;
    type Resolver = Result<T::Resolver, U::Resolver>;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        match resolver {
            Ok(resolver) => {
                let out = out.cast::<ArchivedResultVariantOk<T::Archived>>();
                ptr::addr_of_mut!((*out).0).write(ArchivedResultTag::Ok);

                let (fp, fo) = out_field!(out.1);
                match self.as_ref() {
                    Ok(value) => value.resolve(pos + fp, resolver, fo),
                    Err(_) => unreachable_unchecked(),
                }
            }
            Err(resolver) => {
                let out = out.cast::<ArchivedResultVariantErr<U::Archived>>();
                ptr::addr_of_mut!((*out).0).write(ArchivedResultTag::Err);

                let (fp, fo) = out_field!(out.1);
                match self.as_ref() {
                    Ok(_) => unreachable_unchecked(),
                    Err(err) => err.resolve(pos + fp, resolver, fo),
                }
            }
        }
    }
}

impl<T: Serialize<S>, U: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Result<T, U>
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
