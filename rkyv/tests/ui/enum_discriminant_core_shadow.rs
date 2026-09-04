#![allow(unused_imports)]

extern crate core as real_core;
extern crate self as core;

pub use real_core::*;

pub mod prelude {
    pub mod v1 {
        pub use rustversion::nightly as derive;
    }
}

#[real_core::prelude::v1::derive(rkyv::Archive, rkyv::Serialize)]
#[rkyv(derive(Debug))]
#[repr(u8)]
enum Example {
    First = 1,
    Second,
}

fn main() {
    let _ = ArchivedExample::First;
}
