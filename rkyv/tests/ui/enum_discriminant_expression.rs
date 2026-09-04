trait Discriminant {
    const VALUE: u8;
}

impl<T> Discriminant for T {
    const VALUE: u8 = 11;
}

#[derive(rkyv::Archive)]
#[repr(u8)]
enum Contextual {
    Zero = 0,
    Selected = Self::VALUE,
    Other = 10,
}

impl Contextual {
    const VALUE: u8 = 7;
}

impl ArchivedContextual {
    const VALUE: u8 = 13;
}

fn main() {}
