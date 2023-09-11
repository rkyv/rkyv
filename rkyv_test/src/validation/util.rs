#[cfg(feature = "alloc")]
pub mod alloc {
    use crate::util::alloc::*;
    use rkyv::{
        check_archived_root, ser::Serializer,
        validation::validators::DefaultValidator, CheckBytes, Serialize,
    };

    pub fn serialize_and_check<T: Serialize<DefaultSerializer>>(value: &T)
    where
        T::Archived: for<'a> CheckBytes<DefaultValidator<'a>>,
    {
        let mut serializer = DefaultSerializer::default();
        serializer
            .serialize_value(value)
            .expect("failed to archive value");
        let buf = serializer.into_serializer().into_inner();

        check_archived_root::<T>(buf.as_ref()).unwrap();
    }
}
