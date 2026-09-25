#[derive(rkyv::Archive)]
#[repr(i8)]
enum Negative {
    A = -1,
    B = 0,
}

fn main() {}
