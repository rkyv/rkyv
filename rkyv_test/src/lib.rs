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
    use rkyv::rancor::Panic;

    use crate::util::core::test_archive;

    #[test]
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
    fn derive_partial_ord_struct() {
        use rkyv::{Archive, Deserialize, Serialize};

        #[derive(
            Archive, Deserialize, Serialize, Debug, PartialEq, PartialOrd,
        )]
        #[archive(compare(PartialEq, PartialOrd))]
        #[archive_attr(derive(Debug))]
        pub struct Foo {
            a: i32,
        }

        let small = Foo { a: 0 };
        let big = Foo { a: 1 };
        assert!(small < big);

        let big_bytes =
            rkyv::to_bytes::<Panic>(&big).expect("failed to serialize value");
        let big_archived =
            unsafe { rkyv::access_unchecked::<ArchivedFoo>(&big_bytes) };

        assert!((&small as &dyn PartialOrd<ArchivedFoo>) < big_archived);
    }

    #[test]
    fn derive_partial_ord_enum() {
        use rkyv::{Archive, Deserialize, Serialize};

        #[derive(
            Archive, Deserialize, Serialize, Debug, PartialEq, PartialOrd,
        )]
        #[archive(compare(PartialEq, PartialOrd))]
        #[archive_attr(derive(Debug))]
        pub enum Foo {
            A { a: i32 },
        }

        let small = Foo::A { a: 0 };
        let big = Foo::A { a: 1 };
        assert!(small < big);

        let big_bytes =
            rkyv::to_bytes::<Panic>(&big).expect("failed to serialize value");
        let big_archived =
            unsafe { rkyv::access_unchecked::<ArchivedFoo>(&big_bytes) };

        assert!((&small as &dyn PartialOrd<ArchivedFoo>) < big_archived);
    }
}
