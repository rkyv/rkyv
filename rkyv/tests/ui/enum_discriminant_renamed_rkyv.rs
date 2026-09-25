#![allow(unused_imports)]

extern crate rkyv as real_rkyv;

use rustversion::nightly as derive;

#[::core::prelude::v1::derive(
    real_rkyv::Archive,
    real_rkyv::Serialize,
)]
#[rkyv(crate = real_rkyv, derive(Debug))]
#[repr(u8)]
enum Example {
    First = 1,
    Second,
}

fn main() {
    let _ = ArchivedExample::First;
}
