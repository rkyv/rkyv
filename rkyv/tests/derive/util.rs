use core::fmt::Debug;

use rancor::Panic;
use rkyv::{
    access_unchecked,
    de::{DefaultDeserializer, Pool},
    deserialize,
    ser::DefaultSerializer,
    to_bytes,
    util::AlignedVec,
    Deserialize, Serialize,
};

pub type TestSerializer<'a> = DefaultSerializer<'a, AlignedVec, Panic>;
pub type TestDeserializer = DefaultDeserializer<Panic>;

pub fn roundtrip_with<T>(value: &T, cmp: impl Fn(&T, &T::Archived))
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug + Deserialize<T, TestDeserializer>,
{
    let bytes = to_bytes(value).expect("failed to serialize value");
    let archived_value = unsafe { access_unchecked(&bytes) };
    cmp(value, archived_value);
    let deserialized =
        deserialize::<T, _, Panic>(archived_value, &mut Pool::new())
            .expect("failed to deserialize value");
    assert_eq!(value, &deserialized);
}

pub fn roundtrip<T>(value: &T)
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug + PartialEq<T> + Deserialize<T, TestDeserializer>,
{
    roundtrip_with(value, |a, b| assert_eq!(b, a));
}
