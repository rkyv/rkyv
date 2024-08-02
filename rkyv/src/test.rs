#[cfg(not(feature = "alloc"))]
mod detail {
    use core::mem::MaybeUninit;

    use rancor::Panic;

    use crate::{
        de::{CoreDeserializer, Unpool},
        ser::{
            allocator::SubAllocator, sharing::Unshare, writer::Buffer,
            CoreSerializer,
        },
        util::{serialize_into, Align},
    };

    pub type TestSerializer<'a> = CoreSerializer<'a, Buffer<'a>, Panic>;
    pub type TestDeserializer = CoreDeserializer<Panic>;

    pub fn to_bytes<T>(value: &T, f: impl FnOnce(&mut [u8]))
    where
        T: for<'a> Serialize<TestSerializer<'a>>,
    {
        let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
        let mut scratch = [MaybeUninit::<u8>::uninit(); 256];

        let mut buffer = serialize_into(
            value,
            Serializer::new(
                Buffer::from(&mut *output),
                SubAllocator::new(&mut scratch),
                Unshare,
            ),
        )
        .expect("failed to serialize value")
        .into_writer();

        f(&mut buffer);
    }

    pub fn deserialize<T>(value: &T::Archived) -> T
    where
        T: Archive,
        T::Archived: Deserialize<T, TestDeserializer>,
    {
        crate::deserialize::<T, _, Panic>(value, &mut Unpool)
            .expect("failed to deserialize value")
    }
}

#[cfg(feature = "alloc")]
mod detail {
    use rancor::Panic;

    use crate::{
        de::{DefaultDeserializer, Pool},
        ser::DefaultSerializer,
        util::AlignedVec,
        Archive, Deserialize, Serialize,
    };

    pub type TestSerializer<'a> = DefaultSerializer<'a, AlignedVec, Panic>;
    pub type TestDeserializer = DefaultDeserializer<Panic>;

    pub fn to_bytes<T>(value: &T, f: impl FnOnce(&mut [u8]))
    where
        T: for<'a> Serialize<TestSerializer<'a>>,
    {
        f(&mut crate::to_bytes(value).expect("failed to serialize value"));
    }

    pub fn deserialize<T>(value: &T::Archived) -> T
    where
        T: Archive,
        T::Archived: Deserialize<T, TestDeserializer>,
    {
        crate::deserialize::<T, _, Panic>(value, &mut Pool::new())
            .expect("failed to deserialize value")
    }
}

use core::fmt::Debug;
use std::pin::Pin;

use bytecheck::CheckBytes;
use rancor::{Panic, Strategy};

use self::detail::{deserialize, to_bytes, TestDeserializer, TestSerializer};
use crate::{
    access_mut, validation::validators::DefaultValidator, Deserialize,
    Serialize,
};

pub fn to_archived<T>(value: &T, f: impl FnOnce(Pin<&mut T::Archived>))
where
    T: for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: for<'a> CheckBytes<Strategy<DefaultValidator<'a>, Panic>>,
{
    to_bytes(value, |bytes| {
        let archived_value = access_mut::<T::Archived, Panic>(bytes).unwrap();
        f(archived_value);
    });
}

pub fn roundtrip_with<T>(value: &T, cmp: impl Fn(&T, &T::Archived))
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug
        + Deserialize<T, TestDeserializer>
        + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, Panic>>,
{
    to_archived(value, |archived_value| {
        cmp(value, &*archived_value);
        let deserialized = deserialize::<T>(&*archived_value);
        assert_eq!(value, &deserialized);
    });
}

pub fn roundtrip<T>(value: &T)
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a>>,
    T::Archived: Debug
        + PartialEq<T>
        + Deserialize<T, TestDeserializer>
        + for<'a> CheckBytes<Strategy<DefaultValidator<'a>, Panic>>,
{
    roundtrip_with(value, |a, b| assert_eq!(b, a));
}
