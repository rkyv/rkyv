#[cfg(test)]
mod tests {
    use rkyv::{Archive, CheckBytes};

    #[derive(Archive)]
    #[archive_attr(derive(Debug, Default), repr(C))]
    #[archive(check_bytes)]
    struct Test {
        a: u8,
        b: bool,
    }

    #[derive(Archive)]
    #[archive_attr(
        derive(Debug, Default),
        check_bytes(bound = "__C: Default"),
        repr(C)
    )]
    #[archive(check_bytes)]
    struct OtherAttr {
        a: u8,
        b: bool,
    }

    #[derive(Archive)]
    #[archive_attr(
        derive(CheckBytes, Debug, Default),
        check_bytes(crate = "rkyv::bytecheck"),
        repr(C)
    )]
    struct ExplicitCrate {
        a: u8,
        b: bool,
    }

    #[derive(Archive)]
    #[archive(check_bytes)]
    struct Unit;

    #[derive(Archive)]
    #[archive(check_bytes)]
    struct NewType(u8);

    #[derive(Archive)]
    #[archive(check_bytes)]
    struct Tuple(u8, bool);

    #[derive(Archive)]
    #[archive(check_bytes)]
    #[allow(dead_code)]
    enum Enum {
        A(u8),
        B,
    }

    mod rkyv_path {
        // Doesn't hide it from users of `::rkyv`, but tests that we add the
        // attribute when `rkyv2::CheckBytes` is derived instead of `rkyv::CheckBytes`.
        mod rkyv {}
        use ::rkyv as rkyv2;
        #[derive(rkyv2::Archive)]
        #[archive(crate = "rkyv2", check_bytes)]
        struct RkyvPath;
    }

    #[repr(C, align(16))]
    struct Aligned<const N: usize>([u8; N]);

    #[test]
    fn test() {
        let a = ArchivedTest::default();
        unsafe {
            ArchivedTest::check_bytes(&a, &mut ()).unwrap();
        }
        unsafe {
            let a_bytes: *const u8 = &a as *const ArchivedTest as *const u8;
            let mut bytes = Aligned([*a_bytes.offset(0), *a_bytes.offset(1)]);
            bytes.0[1] = 5;
            ArchivedTest::check_bytes(
                &bytes.0 as *const [u8] as *const ArchivedTest,
                &mut (),
            )
            .unwrap_err();
        }

        // Should throw compile error:
        //     ArchivedOtherAttr::check_bytes(0 as *const _, &mut NoDefault).unwrap();
        //     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `Default` is not implemented for `NoDefault`
        /* unsafe {
            struct NoDefault;
            ArchivedOtherAttr::check_bytes(0 as *const _, &mut NoDefault).unwrap();
        } */
    }
}
