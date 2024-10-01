mod inner {
    use rkyv::{Archive, Serialize};

    #[derive(Archive, Serialize)]
    pub struct TestTuple(pub i32);

    #[derive(Archive, Serialize)]
    pub struct TestStruct {
        pub value: i32,
    }

    #[derive(Archive, Serialize)]
    pub enum TestEnum {
        B(i32),
        C { value: i32 },
    }
}

use inner::{
    ArchivedTestEnum, ArchivedTestStruct, ArchivedTestTuple, TestEnum,
    TestStruct, TestTuple,
};

fn main() {
    TestTuple(42.into());
    ArchivedTestTuple(42.into());
    TestStruct { value: 42.into() };
    ArchivedTestStruct { value: 42.into() };
    TestEnum::B(42.into());
    TestEnum::C { value: 42.into() };
    ArchivedTestEnum::B(42.into());
    ArchivedTestEnum::C { value: 42.into() };
}
