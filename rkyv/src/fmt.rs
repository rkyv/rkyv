use core::fmt;

const PTR_WIDTH: usize = (usize::BITS / 4 + 2) as usize;

pub struct Pointer(pub usize);

impl fmt::Display for Pointer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#0w$x}", self.0, w = PTR_WIDTH)
    }
}
