#[cfg(not(feature = "alloc"))]
mod detail {
    use core::mem::MaybeUninit;

    use rancor::Source;

    use crate::{
        de::{CoreDeserializer, Unpool},
        ser::{
            allocator::SubAllocator, sharing::Unshare, writer::Buffer,
            CoreSerializer,
        },
        util::{serialize_into, Align},
    };

    pub type TestSerializer<'a, E> = CoreSerializer<'a, Buffer<'a>, E>;
    pub type TestDeserializer<E> = CoreDeserializer<E>;

    pub fn to_bytes<T, E>(value: &T, f: impl FnOnce(&[u8]))
    where
        T: for<'a> Serialize<TestSerializer<'a, E>>,
        E: Source,
    {
        let mut output = Align([MaybeUninit::<u8>::uninit(); 256]);
        let mut scratch = [MaybeUninit::<u8>::uninit(); 256];

        let buffer = serialize_into(
            value,
            Serializer::new(
                Buffer::from(&mut *output),
                SubAllocator::new(&mut scratch),
                Unshare,
            ),
        )
        .expect("failed to serialize value")
        .into_writer();

        f(&buffer);
    }

    pub fn deserialize<T, E>(value: &T::Archived) -> T
    where
        T: Archive,
        T::Archived: Deserialize<T, TestDeserializer<E>>,
        E: Source,
    {
        crate::deserialize::<T, _, E>(value, &mut Unpool)
            .expect("failed to deserialize value")
    }
}

#[cfg(feature = "alloc")]
mod detail {
    use rancor::Source;

    use crate::{
        de::{DefaultDeserializer, Pool},
        ser::DefaultSerializer,
        util::AlignedVec,
        Archive, Deserialize, Serialize,
    };

    pub type TestSerializer<'a, E> = DefaultSerializer<'a, AlignedVec, E>;
    pub type TestDeserializer<E> = DefaultDeserializer<E>;

    pub fn to_bytes<T, E>(value: &T, f: impl FnOnce(&[u8]))
    where
        T: for<'a> Serialize<TestSerializer<'a, E>>,
        E: Source,
    {
        f(&crate::to_bytes(value).expect("failed to serialize value"));
    }

    pub fn deserialize<T, E>(value: &T::Archived) -> T
    where
        T: Archive,
        T::Archived: Deserialize<T, TestDeserializer<E>>,
        E: Source,
    {
        crate::deserialize::<T, _, E>(value, &mut Pool::new())
            .expect("failed to deserialize value")
    }
}

use core::fmt::Debug;

pub use detail::{deserialize, to_bytes, TestDeserializer, TestSerializer};
use rancor::Panic;

use crate::{access_unchecked, Deserialize, Serialize};

pub fn roundtrip_with<T>(value: &T, cmp: impl Fn(&T, &T::Archived))
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a, Panic>>,
    T::Archived: Debug + Deserialize<T, TestDeserializer<Panic>>,
{
    to_bytes(value, |bytes| {
        let archived_value = unsafe { access_unchecked(bytes) };
        cmp(value, archived_value);
        let deserialized = deserialize::<T, Panic>(archived_value);
        assert_eq!(value, &deserialized);
    })
}

pub fn roundtrip<T>(value: &T)
where
    T: Debug + PartialEq + for<'a> Serialize<TestSerializer<'a, Panic>>,
    T::Archived: Debug + PartialEq<T> + Deserialize<T, TestDeserializer<Panic>>,
{
    roundtrip_with(value, |a, b| assert_eq!(b, a));
}
