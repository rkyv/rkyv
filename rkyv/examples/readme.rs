use rkyv::{
    deserialize, rancor::Error, util::serialize_into, Archive, Deserialize,
    Serialize,
};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(
    // This will generate a PartialEq impl between our unarchived
    // and archived types
    compare(PartialEq),
    // bytecheck can be used to validate your data if you want. To
    // use the safe API, you have to derive CheckBytes for the
    // archived type
    check_bytes,
)]
// Derives can be passed through to the generated type:
#[rkyv_derive(Debug)]
struct Test {
    int: u8,
    string: String,
    option: Option<Vec<i32>>,
}

fn main() {
    let value = Test {
        int: 42,
        string: "hello world".to_string(),
        option: Some(vec![1, 2, 3, 4]),
    };

    // Serializing is as easy as a single function call
    let _bytes = rkyv::to_bytes::<Error>(&value).unwrap();

    // Or you can customize your serialization for better performance
    // and compatibility with #![no_std] environments
    use rkyv::{
        ser::{allocator::Arena, sharing::Share, Serializer},
        util::AlignedVec,
    };

    let mut arena = Arena::new();
    let serializer = serialize_into::<_, Error>(
        &value,
        Serializer::new(AlignedVec::<16>::new(), arena.acquire(), Share::new()),
    )
    .unwrap();
    let bytes = serializer.into_writer();

    // You can use the safe API for fast zero-copy deserialization
    let archived = rkyv::access::<ArchivedTest, Error>(&bytes[..]).unwrap();
    assert_eq!(archived, &value);

    // Or you can use the unsafe API for maximum performance
    let archived =
        unsafe { rkyv::access_unchecked::<ArchivedTest>(&bytes[..]) };
    assert_eq!(archived, &value);

    // And you can always deserialize back to the original type
    let deserialized =
        deserialize::<Test, _, Error>(archived, &mut ()).unwrap();
    assert_eq!(deserialized, value);
}
