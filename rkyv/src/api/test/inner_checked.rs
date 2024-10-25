use core::fmt::Debug;

use bytecheck::CheckBytes;
use rancor::Panic;

#[cfg(feature = "alloc")]
use crate::api::high::{access_mut, HighValidator as TestValidator};
#[cfg(not(feature = "alloc"))]
use crate::api::low::{access_mut, LowValidator as TestValidator};
use crate::{
    api::test::{deserialize, to_bytes, TestDeserializer, TestSerializer},
    seal::Seal,
    Archive, Deserialize, Serialize,
};

/// Serializes the given type to bytes, accesses the archived version, and calls
/// the given function with it.
pub fn to_archived<T>(value: &T, f: impl FnOnce(Seal<'_, T::Archived>))
where
    T: for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: for<'a> CheckBytes<TestValidator<'a, Panic>>,
{
    to_bytes(value, |bytes| to_archived_from_bytes::<T>(bytes, f));
}

/// Accesses the archived version and calls the given function with it.
pub fn to_archived_from_bytes<T>(
    bytes: &mut [u8],
    f: impl FnOnce(Seal<'_, T::Archived>),
) where
    T: Archive,
    T::Archived: for<'a> CheckBytes<TestValidator<'a, Panic>>,
{
    let archived_value = access_mut::<T::Archived, Panic>(bytes).unwrap();
    f(archived_value);
}

/// Serializes and deserializes the given value, checking for equality with the
/// archived and deserialized values using the given comparison function.
pub fn roundtrip_with<T>(value: &T, cmp: impl Fn(&T, &T::Archived))
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug
        + Deserialize<T, TestDeserializer>
        + for<'a> CheckBytes<TestValidator<'a, Panic>>,
{
    to_archived(value, |archived_value| {
        cmp(value, &*archived_value);
        let deserialized = deserialize::<T>(&*archived_value);
        assert_eq!(value, &deserialized);
    });
}

/// Serializes and deserializes the given value, checking for equality with the
/// archived and deserialized values.
pub fn roundtrip<T>(value: &T)
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug
        + PartialEq<T>
        + Deserialize<T, TestDeserializer>
        + for<'a> CheckBytes<TestValidator<'a, Panic>>,
{
    roundtrip_with(value, |a, b| assert_eq!(b, a));
}
