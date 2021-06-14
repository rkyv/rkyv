#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "arbitrary_enum_discriminant",
    feature(arbitrary_enum_discriminant)
)]

#[cfg(feature = "alloc")]
mod alloc;
pub mod util;
#[cfg(feature = "std")]
mod std;
#[cfg(feature = "validation")]
mod validation;

#[cfg(test)]
mod tests {
    use crate::util::*;

    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_primitives() {
        test_archive(&());
        test_archive(&true);
        test_archive(&false);
        test_archive(&1234567f32);
        test_archive(&12345678901234f64);
        test_archive(&123i8);
        test_archive(&123456i32);
        test_archive(&1234567890i128);
        test_archive(&123u8);
        test_archive(&123456u32);
        test_archive(&1234567890u128);
        #[cfg(not(any(feature = "strict", feature = "archive_le", feature = "archive_be")))]
        test_archive(&(24, true, 16f32));
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Some(42));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_refs() {
        #[cfg(not(feature = "strict"))]
        test_archive_ref::<[i32; 4]>(&[1, 2, 3, 4]);
        test_archive_ref::<str>("hello world");
        test_archive_ref::<[i32]>([1, 2, 3, 4].as_ref());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_slices() {
        test_archive_ref::<str>("hello world");
        test_archive_ref::<[i32]>([1, 2, 3, 4].as_ref());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_empty_slice() {
        test_archive_ref::<[i32; 0]>(&[]);
        test_archive_ref::<[i32]>([].as_ref());
        test_archive_ref::<str>("");
    }
}
