macro_rules! discriminant {
    ($ty:ty) => {
        <$ty as Discriminant>::VALUE
    };
}

trait Discriminant {
    const VALUE: u8;
}

impl<T> Discriminant for T {
    const VALUE: u8 = 1;
}

#[derive(rkyv::Archive)]
#[repr(u8)]
enum MacroWrapped {
    A = discriminant!(Self),
    B = 2,
}

fn main() {}
