trait Discriminant {
    const VALUE: u8;
}

impl<T> Discriminant for T {
    const VALUE: u8 = 1;
}

#[derive(rkyv::Archive)]
#[repr(u8)]
enum Qualified {
    A = <Self as Discriminant>::VALUE,
    B = 2,
}

fn main() {}
