use rkyv::{rancor::Fallible, DeserializeUnsized};

struct Value(core::num::NonZeroU8);
struct ArchivedValue;

impl<D: Fallible + ?Sized> DeserializeUnsized<Value, D> for ArchivedValue {
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        _: *mut Value,
    ) -> Result<(), D::Error> {
        Ok(())
    }

    fn deserialize_metadata(&self) {}
}

fn main() {}
