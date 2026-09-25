use rkyv::traits::ArchivePointee;

#[rkyv::ptr_meta::pointee]
trait Example {}

impl ArchivePointee for dyn Example {
    type ArchivedMetadata = ();

    fn pointer_metadata(
        _: &Self::ArchivedMetadata,
    ) -> <Self as rkyv::ptr_meta::Pointee>::Metadata {
        unreachable!()
    }
}

fn main() {}
