use rkyv::{Archive, Deserialize, Serialize};

fn main() {
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
