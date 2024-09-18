use core::mem::MaybeUninit;

use rancor::Panic;

use crate::{
    api::low::{to_bytes_in_with_alloc, LowDeserializer, LowSerializer},
    ser::{allocator::SubAllocator, writer::Buffer},
    util::Align,
    Deserialize, Serialize,
};

/// The serializer type for tests.
pub type TestSerializer<'a> =
    LowSerializer<Buffer<'a>, SubAllocator<'a>, Panic>;
/// The deserializer type for tests.
pub type TestDeserializer = LowDeserializer<Panic>;

/// Serializes the given value to bytes using the test serializer, then calls
/// the given function on those bytes.
pub fn to_bytes<T>(value: &T, f: impl FnOnce(&mut [u8]))
where
    T: for<'a> Serialize<TestSerializer<'a>>,
{
    let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
    let mut alloc = [MaybeUninit::<u8>::uninit(); 256];

    let mut bytes = to_bytes_in_with_alloc(
        value,
        Buffer::from(&mut *output),
        SubAllocator::new(&mut alloc),
    )
    .expect("failed to serialize value");

    f(&mut bytes);
}

/// Deserializes the given value using the test deserializer.
pub fn deserialize<T>(value: &impl Deserialize<T, TestDeserializer>) -> T {
    crate::api::deserialize_using::<T, _, Panic>(value, &mut ())
        .expect("failed to deserialize value")
}
