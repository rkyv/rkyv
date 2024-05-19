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

#[cfg(test)]
mod tests {
    use crate::{
        test::roundtrip, with::Niche, Archive, Deserialize, Serialize,
    };

    #[test]
    fn ambiguous_niched_archived_box() {
        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive_attr(derive(Debug))]
        #[archive(crate)]
        #[archive(compare(PartialEq))]
        struct HasNiche {
            #[with(Niche)]
            inner: Option<Box<[u32]>>,
        }

        roundtrip(&HasNiche {
            inner: Some(Box::<[u32]>::from([])),
        });
        roundtrip(&HasNiche { inner: None });
    }
}
