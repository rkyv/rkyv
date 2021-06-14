mod test_alloc;
#[cfg(feature = "std")]
mod test_std;

pub mod util {
    use bytecheck::CheckBytes;
    use rkyv::{
        check_archived_root,
        ser::{serializers::AlignedSerializer, Serializer},
        util::AlignedVec,
        validation::validators::DefaultArchiveValidator,
        Serialize,
    };

    pub fn serialize_and_check<T: Serialize<AlignedSerializer<AlignedVec>>>(value: &T)
    where
        T::Archived: for<'a> CheckBytes<DefaultArchiveValidator<'a>>,
    {
        let mut serializer = AlignedSerializer::new(AlignedVec::new());
        serializer
            .serialize_value(value)
            .expect("failed to archive value");
        let buf = serializer.into_inner();
        check_archived_root::<T>(buf.as_ref()).unwrap();
    }
}
