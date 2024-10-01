#![allow(non_camel_case_types)]

use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
struct r#virtual {
    r#virtual: i32,
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
#[rkyv(compare(PartialEq), derive(Debug))]
enum r#try {
    r#try { r#try: i32 },
}

fn main() {}
