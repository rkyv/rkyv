#[cfg(feature = "alloc")]
pub mod alloc {
    use rkyv::{
        access,
        bytecheck::CheckBytes,
        rancor::{Source, Strategy},
        validation::validators::DefaultValidator,
        Serialize,
    };

    use crate::util::alloc::*;

    pub fn serialize_and_check<T, E>(value: &T)
    where
        T: Serialize<Strategy<DefaultSerializer, E>>,
        T::Archived: for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
        E: Source,
    {
        let buf =
            rkyv::util::serialize_into(value, DefaultSerializer::default())
                .expect("failed to archive value")
                .into_writer();

        access::<T::Archived, E>(buf.as_ref()).unwrap();
    }
}
