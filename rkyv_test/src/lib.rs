#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(
    feature = "arbitrary_enum_discriminant",
    feature(arbitrary_enum_discriminant)
)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "alloc")]
mod test_alloc;
#[cfg(feature = "std")]
mod test_std;
pub mod util;
#[cfg(feature = "validation")]
pub mod validation;

#[cfg(test)]
mod tests {
    use crate::util::core::*;

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
        test_archive(&12345i16);
        test_archive(&1234567890i32);
        test_archive(&1234567890123456789i64);
        test_archive(&123456789012345678901234567890123456789i128);
        test_archive(&123u8);
        test_archive(&12345u16);
        test_archive(&1234567890u32);
        test_archive(&12345678901234567890u64);
        test_archive(&123456789012345678901234567890123456789u128);
        #[cfg(not(any(feature = "strict", feature = "archive_le", feature = "archive_be")))]
        test_archive(&(24, true, 16f32));
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Result::<i32, u32>::Ok(12345i32));
        test_archive(&Result::<i32, u32>::Err(12345u32));
        test_archive(&Some(42));

        #[cfg(feature = "rkyv/rend")]
        {
            use rkyv::rend::*;
            test_archive(f32_be::new(&1234567f32));
            test_archive(f64_be::new(&12345678901234f64));
            test_archive(i8_be::new(&123i8));
            test_archive(i16_be::new(&12345i16));
            test_archive(i32_be::new(&1234567890i32));
            test_archive(i64_be::new(&1234567890123456789i64));
            test_archive(i128_be::new(&123456789012345678901234567890123456789i128));
            test_archive(u8_be::new(&123u8));
            test_archive(u16_be::new(&12345u16));
            test_archive(u32_be::new(&1234567890u32));
            test_archive(u64_be::new(&12345678901234567890u64));
            test_archive(u128_be::new(&123456789012345678901234567890123456789u128));

            test_archive(f32_le::new(&1234567f32));
            test_archive(f64_le::new(&12345678901234f64));
            test_archive(i8_le::new(&123i8));
            test_archive(i16_le::new(&12345i16));
            test_archive(i32_le::new(&1234567890i32));
            test_archive(i64_le::new(&1234567890123456789i64));
            test_archive(i128_le::new(&123456789012345678901234567890123456789i128));
            test_archive(u8_le::new(&123u8));
            test_archive(u16_le::new(&12345u16));
            test_archive(u32_le::new(&1234567890u32));
            test_archive(u64_le::new(&12345678901234567890u64));
            test_archive(u128_le::new(&123456789012345678901234567890123456789u128));
        }
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

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_nonzero() {
        use core::num::{
            NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8, NonZeroU128, NonZeroU16,
            NonZeroU32, NonZeroU64, NonZeroU8,
        };

        unsafe {
            test_archive(&NonZeroI8::new_unchecked(123));
            test_archive(&NonZeroI16::new_unchecked(12345));
            test_archive(&NonZeroI32::new_unchecked(1234567890));
            test_archive(&NonZeroI64::new_unchecked(1234567890123456789));
            test_archive(&NonZeroI128::new_unchecked(
                123456789012345678901234567890123456789,
            ));
            test_archive(&NonZeroU8::new_unchecked(123));
            test_archive(&NonZeroU16::new_unchecked(12345));
            test_archive(&NonZeroU32::new_unchecked(1234567890));
            test_archive(&NonZeroU64::new_unchecked(12345678901234567890));
            test_archive(&NonZeroU128::new_unchecked(
                123456789012345678901234567890123456789,
            ));
        }

        #[cfg(feature = "rkyv/rend")]
        unsafe {
            use rkyv::rend::*;
            test_archive(&NonZeroI8_be::new(NonZeroI8::new_unchecked(123)));
            test_archive(&NonZeroI16_be::new(NonZeroI16::new_unchecked(12345)));
            test_archive(&NonZeroI32_be::new(NonZeroI32::new_unchecked(1234567890)));
            test_archive(&NonZeroI64_be::new(NonZeroI64::new_unchecked(
                1234567890123456789,
            )));
            test_archive(&NonZeroI128_be::new(NonZeroI128::new_unchecked(
                123456789012345678901234567890123456789,
            )));
            test_archive(&NonZeroU8_be::new(NonZeroU8::new_unchecked(123)));
            test_archive(&NonZeroU16_be::new(NonZeroU16::new_unchecked(12345)));
            test_archive(&NonZeroU32_be::new(NonZeroU32::new_unchecked(1234567890)));
            test_archive(&NonZeroU64_be::new(NonZeroU64::new_unchecked(
                1234567890123456789,
            )));
            test_archive(&NonZeroU128_be::new(NonZeroU128::new_unchecked(
                123456789012345678901234567890123456789,
            )));

            test_archive(&NonZeroI8_le::new(NonZeroI8::new_unchecked(123)));
            test_archive(&NonZeroI16_le::new(NonZeroI16::new_unchecked(12345)));
            test_archive(&NonZeroI32_le::new(NonZeroI32::new_unchecked(1234567890)));
            test_archive(&NonZeroI64_le::new(NonZeroI64::new_unchecked(
                1234567890123456789,
            )));
            test_archive(&NonZeroI128_le::new(NonZeroI128::new_unchecked(
                123456789012345678901234567890123456789,
            )));
            test_archive(&NonZeroU8_le::new(NonZeroU8::new_unchecked(123)));
            test_archive(&NonZeroU16_le::new(NonZeroU16::new_unchecked(12345)));
            test_archive(&NonZeroU32_le::new(NonZeroU32::new_unchecked(1234567890)));
            test_archive(&NonZeroU64_le::new(NonZeroU64::new_unchecked(
                1234567890123456789,
            )));
            test_archive(&NonZeroU128_le::new(NonZeroU128::new_unchecked(
                123456789012345678901234567890123456789,
            )));
        }
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_zst() {
        use rkyv::{Archive, Deserialize, Serialize};

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct MyZST;

        test_archive::<[MyZST; 0]>(&[]);
        test_archive_ref::<[MyZST]>(&[]);
        test_archive::<[MyZST; 4]>(&[MyZST, MyZST, MyZST, MyZST]);
        test_archive_ref::<[MyZST]>(&[MyZST, MyZST, MyZST, MyZST]);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    #[allow(non_camel_case_types)]
    fn archive_raw_identifiers() {
        use rkyv::{Archive, Deserialize, Serialize};

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct r#virtual {
            r#virtual: i32,
        }

        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        enum r#try {
            r#try { r#try: i32 },
        }

        test_archive::<r#virtual>(&r#virtual { r#virtual: 42 });
        test_archive::<r#try>(&r#try::r#try { r#try: 42 });
    }
}
