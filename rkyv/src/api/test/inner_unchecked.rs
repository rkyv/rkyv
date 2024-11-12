#![allow(dead_code)]

use core::fmt::Debug;

use crate::{
    access_unchecked_mut,
    api::test::{deserialize, to_bytes, TestDeserializer, TestSerializer},
    seal::Seal,
    Archive, Deserialize, Serialize,
};

/// Serializes the given type to bytes, accesses the archived version, and calls
/// the given function with it.
pub fn to_archived<T>(value: &T, f: impl FnOnce(Seal<'_, T::Archived>))
where
    T: for<'a> Serialize<TestSerializer<'a>>,
{
    to_bytes(value, |bytes| to_archived_from_bytes::<T>(bytes, f));
}

/// Accesses the archived version and calls the given function with it.
pub fn to_archived_from_bytes<T>(
    bytes: &mut [u8],
    f: impl FnOnce(Seal<'_, T::Archived>),
) where
    T: Archive,
{
    let archived_value = unsafe { access_unchecked_mut::<T::Archived>(bytes) };
    f(archived_value);
}

/// Serializes and deserializes the given value, checking for equality with the
/// archived and deserialized values using the given comparison function.
pub fn roundtrip_with<T>(value: &T, cmp: impl Fn(&T, &T::Archived))
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug + Deserialize<T, TestDeserializer>,
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
    T::Archived: Debug + PartialEq<T> + Deserialize<T, TestDeserializer>,
{
    roundtrip_with(value, |a, b| assert_eq!(b, a));
}
