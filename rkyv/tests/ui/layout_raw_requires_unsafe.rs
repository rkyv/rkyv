use core::alloc::{Layout, LayoutError};

use rkyv::traits::LayoutRaw;

#[rkyv::ptr_meta::pointee]
trait Example {}

impl LayoutRaw for dyn Example {
    fn layout_raw(
        _: <Self as rkyv::ptr_meta::Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        unreachable!()
    }
}

fn main() {}
