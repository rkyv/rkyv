#![allow(unused_imports)]

use rkyv::Archive as PartialEq;
use rkyv::Archive as PartialOrd;

#[derive(rkyv::Archive)]
#[repr(u8)]
enum Example {
    First = 1,
    Second,
}

fn main() {
    let _ = ArchivedExample::First;
}
