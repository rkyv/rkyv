#[derive(rkyv::Archive)]
#[rkyv(attr(rustversion::nightly))]
#[repr(u8)]
enum ForwardedAttribute {
    A,
    B,
}

fn main() {}
