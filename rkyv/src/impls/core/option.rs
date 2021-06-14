use crate::{
    option::ArchivedOption,
    Archive,
    Deserialize,
    Fallible,
    Serialize,
};
use core::{
    mem::MaybeUninit,
    ptr,
};

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
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        match resolver {
            None => {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedOptionVariantNone>>();
                ptr::addr_of_mut!((*out.as_mut_ptr()).0).write(ArchivedOptionTag::None);
            }
            Some(resolver) => {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedOptionVariantSome<T::Archived>>>();
                ptr::addr_of_mut!((*out.as_mut_ptr()).0).write(ArchivedOptionTag::Some);

                let (fp, fo) = out_field!(out.1);
                self.as_ref().unwrap().resolve(pos + fp, resolver, fo);
            }
        }
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Option<T> {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        self.as_ref()
            .map(|value| value.serialize(serializer))
            .transpose()
    }
}

impl<T: Archive, D: Fallible + ?Sized> Deserialize<Option<T>, D> for ArchivedOption<T::Archived>
where
    T::Archived: Deserialize<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Option<T>, D::Error> {
        match self {
            ArchivedOption::Some(value) => Ok(Some(value.deserialize(deserializer)?)),
            ArchivedOption::None => Ok(None),
        }
    }
}