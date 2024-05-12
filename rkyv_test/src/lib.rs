#![cfg_attr(all(not(feature = "std"), not(feature = "wasm")), no_std)]
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
#[cfg(feature = "bytecheck")]
pub mod validation;

#[cfg(test)]
mod tests {
    use core::mem::MaybeUninit;

    use rkyv::{
        rancor::Error, ser::writer::Buffer, tuple::ArchivedTuple3,
        util::serialize_into, Archived,
    };
    #[cfg(feature = "wasm")]
    use wasm_bindgen_test::*;

    use crate::util::core::{test_archive, test_archive_with};

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
        test_archive_with(
            &(24, true, 16f32),
            |(a, b, c), ArchivedTuple3(d, e, f)| a == d && b == e && c == f,
        );
        test_archive(&[1, 2, 3, 4, 5, 6]);

        test_archive(&Option::<()>::None);
        test_archive(&Result::<i32, u32>::Ok(12345i32));
        test_archive(&Result::<i32, u32>::Err(12345u32));
        test_archive(&Some(42));
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_unsized() {
        test_archive::<Box<[i32; 4]>>(&Box::new([1, 2, 3, 4]));
        test_archive::<Box<str>>(&String::from("hello world").into_boxed_str());
        test_archive::<Box<[i32]>>(&Vec::from([1, 2, 3, 4]).into_boxed_slice());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_empty_unsized() {
        test_archive::<Box<[i32; 0]>>(&Box::new([]));
        test_archive::<Box<str>>(&String::from("").into_boxed_str());
        test_archive::<Box<[i32]>>(&Vec::from([]).into_boxed_slice());
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_nonzero() {
        use core::num::{
            NonZeroI128, NonZeroI16, NonZeroI32, NonZeroI64, NonZeroI8,
            NonZeroU128, NonZeroU16, NonZeroU32, NonZeroU64, NonZeroU8,
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
        test_archive::<Box<[MyZST]>>(&Vec::from([]).into_boxed_slice());
        test_archive::<[MyZST; 4]>(&[MyZST, MyZST, MyZST, MyZST]);
        test_archive::<Box<[MyZST]>>(
            &Vec::from([MyZST, MyZST, MyZST, MyZST]).into_boxed_slice(),
        );
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

        test_archive(&r#virtual { r#virtual: 42 });
        test_archive(&r#try::r#try { r#try: 42 });
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn archive_enum_explicit_discriminants() {
        use rkyv::{Archive, Deserialize, Serialize};

        #[derive(Archive, Deserialize, Serialize)]
        enum Foo {
            A = 2,
            B = 4,
            C = 6,
        }

        assert_eq!(ArchivedFoo::A as usize, 2);
        assert_eq!(ArchivedFoo::B as usize, 4);
        assert_eq!(ArchivedFoo::C as usize, 6);
    }

    #[test]
    #[cfg_attr(feature = "wasm", wasm_bindgen_test)]
    fn buffer_serializer_zeroes_padding() {
        use core::mem::size_of;

        use rkyv::{Archive, Serialize};

        #[derive(Archive, Serialize)]
        pub struct PaddedExample {
            a: u8,
            b: u64,
        }

        let mut bytes = [MaybeUninit::new(0xccu8); 256];
        let buffer = serialize_into::<_, Error>(
            &PaddedExample { a: 0u8, b: 0u64 },
            Buffer::from(&mut bytes),
        )
        .unwrap();
        assert!(&buffer[0..size_of::<Archived<PaddedExample>>()]
            .iter()
            .all(|&b| b == 0));
    }
}
