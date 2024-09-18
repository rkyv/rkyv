use rancor::Panic;

use crate::{
    api::high::{HighDeserializer, HighSerializer},
    de::Pool,
    ser::allocator::ArenaHandle,
    util::AlignedVec,
    Deserialize, Serialize,
};

/// The serializer type for tests.
pub type TestSerializer<'a> =
    HighSerializer<AlignedVec, ArenaHandle<'a>, Panic>;
/// The deserializer type for tests.
pub type TestDeserializer = HighDeserializer<Panic>;

/// Serializes the given value to bytes using the test serializer, then calls
/// the given function on those bytes.
pub fn to_bytes<T>(value: &T, f: impl FnOnce(&mut [u8]))
where
    T: for<'a> Serialize<TestSerializer<'a>>,
{
    let mut bytes =
        crate::api::high::to_bytes(value).expect("failed to serialize value");
    f(&mut bytes);
}

/// Deserializes the given value using the test deserializer.
pub fn deserialize<T>(value: &impl Deserialize<T, TestDeserializer>) -> T {
    crate::api::deserialize_using::<T, _, Panic>(value, &mut Pool::new())
        .expect("failed to deserialize value")
}
