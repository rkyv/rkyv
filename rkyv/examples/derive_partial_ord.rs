fn main() {
    use rancor::Panic;
    use rkyv::{Archive, Deserialize, Serialize};

    // Struct

    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq, PartialOrd)]
    #[rkyv(compare(PartialEq, PartialOrd), derive(Debug))]
    pub enum Struct {
        A { a: i32 },
    }

    let small = Struct::A { a: 0 };
    let big = Struct::A { a: 1 };
    assert!(small < big);

    let big_bytes =
        rkyv::to_bytes::<Panic>(&big).expect("failed to serialize value");
    let big_archived =
        unsafe { rkyv::access_unchecked::<ArchivedStruct>(&big_bytes) };

    assert!((&small as &dyn PartialOrd<ArchivedStruct>) < big_archived);

    // Enum

    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq, PartialOrd)]
    #[rkyv(compare(PartialEq, PartialOrd), derive(Debug))]
    pub struct Enum {
        a: i32,
    }

    let small = Enum { a: 0 };
    let big = Enum { a: 1 };
    assert!(small < big);

    let big_bytes =
        rkyv::to_bytes::<Panic>(&big).expect("failed to serialize value");
    let big_archived =
        unsafe { rkyv::access_unchecked::<ArchivedEnum>(&big_bytes) };

    assert!((&small as &dyn PartialOrd<ArchivedEnum>) < big_archived);
}
