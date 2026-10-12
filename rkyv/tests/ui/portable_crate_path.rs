mod contract {
    /// # Safety
    ///
    /// Implementing types must have alignment one.
    pub unsafe trait Portable {}
}

#[derive(rkyv::Portable)]
#[rkyv(crate = contract)]
#[repr(C)]
struct OverAligned;

fn main() {}
