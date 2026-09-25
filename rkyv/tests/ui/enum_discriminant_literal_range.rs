#[derive(rkyv::Archive)]
#[repr(u16)]
enum OutOfRange {
    A = 256,
}

fn main() {}
