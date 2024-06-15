use rancor::Panic;
use rkyv::{Archive, Deserialize, Serialize};

#[test]
fn explicit_enum_discriminant() {
    #[derive(Archive, Deserialize, Serialize)]
    enum Foo {
        A = 2,
        B = 4,
        C = 6,
    }

    assert_eq!(ArchivedFoo::A as usize, 2);
    assert_eq!(ArchivedFoo::B as usize, 4);
    assert_eq!(ArchivedFoo::C as usize, 6);
}

#[test]
fn partial_ord_struct() {
    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq, PartialOrd)]
    #[archive(compare(PartialEq, PartialOrd))]
    #[archive_attr(derive(Debug))]
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
}

#[test]
fn partial_ord_enum() {
    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq, PartialOrd)]
    #[archive(compare(PartialEq, PartialOrd))]
    #[archive_attr(derive(Debug))]
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

#[test]
fn raw_identifiers() {
    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
    #[archive(compare(PartialEq))]
    #[archive_attr(derive(Debug))]
    struct r#virtual {
        r#virtual: i32,
    }
    
    #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
    #[archive(compare(PartialEq))]
    #[archive_attr(derive(Debug))]
    enum r#try {
        r#try { r#try: i32 },
    }

    roundtrip(&r#virtual { r#virtual: 42 });
    roundtrip(&r#try::r#try { r#try: 42 });
}
