#[cfg(test)]
mod tests {
    use rkyv::{
        bytecheck::{check_bytes, CheckBytes},
        rancor::Failure,
        Archive,
    };

    #[derive(Archive)]
    #[archive_attr(derive(Debug, Default))]
    #[archive(check_bytes)]
    struct Test {
        a: u8,
        b: bool,
    }

    #[derive(Archive)]
    #[archive_attr(
        derive(Debug, Default),
        check_bytes(bounds(__C: Default)),
    )]
    #[archive(check_bytes)]
    struct OtherAttr {
        a: u8,
        b: bool,
    }

    #[derive(Archive)]
    #[archive_attr(
        derive(CheckBytes, Debug, Default),
        check_bytes(crate = "rkyv::bytecheck")
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
        // attribute when `rkyv2::CheckBytes` is derived instead of
        // `rkyv::CheckBytes`.
        mod rkyv {}
        use ::rkyv as rkyv2;
        #[derive(rkyv2::Archive)]
        #[archive(crate = rkyv2, check_bytes)]
        struct RkyvPath;
    }

    #[repr(C, align(16))]
    struct Aligned<const N: usize>([u8; N]);

    #[test]
    fn test() {
        let a = ArchivedTest::default();
        unsafe {
            check_bytes::<_, Failure>(&a).unwrap();
        }
        unsafe {
            let a_bytes: *const u8 = &a as *const ArchivedTest as *const u8;
            let mut bytes = Aligned([*a_bytes.offset(0), *a_bytes.offset(1)]);
            bytes.0[1] = 5;
            check_bytes::<_, Failure>(
                &bytes.0 as *const [u8] as *const ArchivedTest,
            )
            .unwrap_err();
        }
    }
}
