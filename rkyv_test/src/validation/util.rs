#[cfg(feature = "alloc")]
pub mod alloc {
    use rkyv::{
        access,
        bytecheck::CheckBytes,
        rancor::{Source, Strategy},
        to_bytes,
        validation::validators::DefaultValidator,
        Serialize,
    };

    use crate::util::alloc::*;

    pub fn serialize_and_check<T, E>(value: &T)
    where
        T: for<'a> Serialize<DefaultSerializer<'a, E>>,
        T::Archived: for<'a> CheckBytes<Strategy<DefaultValidator<'a>, E>>,
        E: Source,
    {
        let buf = to_bytes::<E>(value).expect("failed to archive value");

        access::<T::Archived, E>(buf.as_ref()).unwrap();
    }
}
