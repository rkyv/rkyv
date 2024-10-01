use rancor::{Failure, Strategy};
use rkyv::{access_unchecked, Archive, Serialize, Deserialize};

pub trait MyTrait {}

struct Serializer;

impl MyTrait for Serializer {}

struct NotSerializer;

#[derive(Archive, Serialize, Deserialize)]
#[rkyv(deserialize_bounds(__D: MyTrait))]
pub struct MyStruct;

fn main() {
    let bytes = &[];
    let archived = unsafe {
        access_unchecked::<ArchivedMyStruct>(bytes)
    };
    let state = archived.deserialize(Strategy::<_, Failure>::wrap(&mut NotSerializer));
}
