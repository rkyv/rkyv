#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use crate::{niche::option_box::ArchivedOptionBox, ArchivePointee};

impl<T, U> PartialEq<Option<Box<T>>> for ArchivedOptionBox<U>
where
    T: ?Sized,
    U: ArchivePointee + PartialEq<T> + ?Sized,
{
    fn eq(&self, other: &Option<Box<T>>) -> bool {
        match (self.as_deref(), other.as_deref()) {
            (Some(self_value), Some(other_value)) => self_value.eq(other_value),
            (None, None) => true,
            _ => false,
        }
    }
}

#[cfg(feature = "extra_traits")]
impl<T, U> PartialEq<ArchivedOptionBox<T>> for Option<Box<U>>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    fn eq(&self, other: &ArchivedOptionBox<T>) -> bool {
        other.eq(self)
    }
}
