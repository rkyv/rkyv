#![cfg_attr(feature = "const_generics", feature(const_generics))]
#![cfg_attr(feature = "const_generics", allow(incomplete_features))]

mod core;
mod error;
#[cfg(not(feature = "no_std"))]
mod std;

pub use crate::error::{
    Error,
    Result,
};
#[cfg(not(feature = "no_std"))]
pub use crate::std::ArchiveWriter;

pub trait Archive {
    type Archived;

    fn archive(&self) -> Self::Archived;
    fn unarchive(archived: &Self::Archived) -> Self;
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
