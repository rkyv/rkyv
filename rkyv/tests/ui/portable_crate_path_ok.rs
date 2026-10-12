#[derive(rkyv::Portable)]
#[rkyv(crate = ::rkyv)]
#[repr(C)]
struct ExplicitCratePath {
    value: u8,
}

fn main() {}
