#![allow(unused_imports)]

use rustversion::nightly as derive;

#[::core::prelude::v1::derive(rkyv::Archive, rkyv::Serialize)]
#[repr(u8)]
enum GeneratedOnly {
    First = 1,
    Second,
}

#[::core::prelude::v1::derive(rkyv::Archive)]
#[rkyv(derive(Debug))]
#[repr(u8)]
enum ForwardedDerive {
    First = 1,
    Second,
}

fn main() {
    let _ = ArchivedGeneratedOnly::First;
    let _ = ArchivedForwardedDerive::First;
}
