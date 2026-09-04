use core::num::NonZeroU8;

use rkyv::{rancor::Fallible, DeserializeUnsized};

#[allow(dead_code)]
struct Value(NonZeroU8);

struct ArchivedValue;

// SAFETY:
// - The metadata for sized `Value` is `()`.
// - On success, the implementation writes a valid `Value` to `out`.
// - The implementation cannot return an error or unwind, and it does not
//   deallocate or retain access to the output storage.
unsafe impl<D: Fallible + ?Sized> DeserializeUnsized<Value, D>
    for ArchivedValue
{
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        out: *mut Value,
    ) -> Result<(), D::Error> {
        // SAFETY: The caller provides storage valid for writing a `Value`.
        unsafe { out.write(Value(NonZeroU8::MIN)) };
        Ok(())
    }

    fn deserialize_metadata(&self) {}
}

fn main() {}
