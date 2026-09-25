#![allow(unused_imports)]

use rustversion::nightly as bytecheck;

#[derive(rkyv::Archive)]
#[repr(u8)]
enum Example {
    First = 1,
    Second,
}

fn main() {
    let _ = ArchivedExample::First;
}
