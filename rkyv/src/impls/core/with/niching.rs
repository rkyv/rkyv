use core::num::{NonZeroI8, NonZeroU8};

use crate::{
    boxed::ArchivedBox,
    niche::{
        niched_option::NichedOption,
        niching::{
            Bool, DefaultNiche, NaN, Niching, Null, SharedNiching, Zero,
        },
    },
    primitive::{
        ArchivedF32, ArchivedF64, ArchivedI128, ArchivedI16, ArchivedI32,
        ArchivedI64, ArchivedNonZeroI128, ArchivedNonZeroI16,
        ArchivedNonZeroI32, ArchivedNonZeroI64, ArchivedNonZeroU128,
        ArchivedNonZeroU16, ArchivedNonZeroU32, ArchivedNonZeroU64,
        ArchivedU128, ArchivedU16, ArchivedU32, ArchivedU64,
    },
    traits::ArchivePointee,
    Place, Portable, RelPtr,
};

macro_rules! impl_default_niche {
    ($ty:ty, $niche:ty) => {
        impl Niching<$ty> for DefaultNiche {
            unsafe fn is_niched(niched: *const $ty) -> bool {
                unsafe { <$niche as Niching<$ty>>::is_niched(niched) }
            }

            fn resolve_niched(out: Place<$ty>) {
                <$niche as Niching<$ty>>::resolve_niched(out)
            }
        }
    };
}

// Zero

macro_rules! impl_nonzero_zero_niching {
    ($nz:ty, $int:ty) => {
        impl Niching<$nz> for Zero {
            unsafe fn is_niched(niched: *const $nz) -> bool {
                let value = unsafe { &*niched.cast::<$int>() };
                *value == 0
            }

            fn resolve_niched(out: Place<$nz>) {
                let out = unsafe { out.cast_unchecked::<$int>() };
                out.write(0.into());
            }
        }

        impl_default_niche!($nz, Zero);
    };
}

impl_nonzero_zero_niching!(NonZeroU8, u8);
impl_nonzero_zero_niching!(ArchivedNonZeroU16, ArchivedU16);
impl_nonzero_zero_niching!(ArchivedNonZeroU32, ArchivedU32);
impl_nonzero_zero_niching!(ArchivedNonZeroU64, ArchivedU64);
impl_nonzero_zero_niching!(ArchivedNonZeroU128, ArchivedU128);

impl_nonzero_zero_niching!(NonZeroI8, i8);
impl_nonzero_zero_niching!(ArchivedNonZeroI16, ArchivedI16);
impl_nonzero_zero_niching!(ArchivedNonZeroI32, ArchivedI32);
impl_nonzero_zero_niching!(ArchivedNonZeroI64, ArchivedI64);
impl_nonzero_zero_niching!(ArchivedNonZeroI128, ArchivedI128);

// NaN

macro_rules! impl_float_nan_niching {
    ($fl:ty, $ar:ty) => {
        impl Niching<$ar> for NaN {
            unsafe fn is_niched(niched: *const $ar) -> bool {
                unsafe { (*niched).to_native().is_nan() }
            }

            fn resolve_niched(out: Place<$ar>) {
                out.write(<$fl>::NAN.into());
            }
        }
    };
}

impl_float_nan_niching!(f32, ArchivedF32);
impl_float_nan_niching!(f64, ArchivedF64);

// Bool

impl Niching<bool> for Bool {
    unsafe fn is_niched(niched: *const bool) -> bool {
        unsafe { (*niched.cast::<u8>()) > 1 }
    }

    fn resolve_niched(out: Place<bool>) {
        unsafe { out.cast_unchecked::<u8>().write(2) };
    }
}

impl_default_niche!(bool, Bool);

// Null

impl<T> Niching<ArchivedBox<T>> for Null
where
    T: ArchivePointee + Portable + ?Sized,
{
    unsafe fn is_niched(niched: *const ArchivedBox<T>) -> bool {
        unsafe { (*niched.cast::<RelPtr<T>>()).is_invalid() }
    }

    fn resolve_niched(out: Place<ArchivedBox<T>>) {
        let out = unsafe { out.cast_unchecked::<RelPtr<T>>() };
        RelPtr::emplace_invalid(out);
    }
}

impl<T> Niching<ArchivedBox<T>> for DefaultNiche
where
    T: ArchivePointee + Portable + ?Sized,
{
    unsafe fn is_niched(niched: *const ArchivedBox<T>) -> bool {
        unsafe { <Null as Niching<ArchivedBox<T>>>::is_niched(niched) }
    }

    fn resolve_niched(out: Place<ArchivedBox<T>>) {
        <Null as Niching<ArchivedBox<T>>>::resolve_niched(out);
    }
}

// SharedNiching

impl<T, N1, N2> Niching<NichedOption<T, N1>> for N2
where
    T: SharedNiching<N1, N2>,
    N1: Niching<T>,
    N2: Niching<T>,
{
    unsafe fn is_niched(niched: *const NichedOption<T, N1>) -> bool {
        unsafe { <Self as Niching<T>>::is_niched(niched.cast()) }
    }

    fn resolve_niched(out: Place<NichedOption<T, N1>>) {
        <Self as Niching<T>>::resolve_niched(unsafe { out.cast_unchecked() })
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;

    use crate::{
        api::test::{
            deserialize, roundtrip_with, to_archived, to_archived_from_bytes,
            to_bytes,
        },
        boxed::ArchivedBox,
        niche::niching::{DefaultNiche, NaN, Zero},
        with::{AsBox, MapNiche, NicheInto},
        Archive, Deserialize, Serialize,
    };

    #[test]
    fn with_struct() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Nichable {
            #[rkyv(niche = NaN)]
            not_nan: f32,
            #[rkyv(niche = Zero)]
            int: NonZeroU32,
            #[rkyv(niche)] // Default = Bool
            boolean: bool,
        }

        impl Nichable {
            fn create() -> Self {
                Nichable {
                    not_nan: 123.456,
                    int: unsafe { NonZeroU32::new_unchecked(789) },
                    boolean: true,
                }
            }
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Middle {
            #[rkyv(with = NicheInto<Zero>, niche = NaN, niche)]
            // Default = Bool
            a: Option<Nichable>,
            #[rkyv(with = NicheInto<NaN>, niche = Zero)]
            b: Option<Nichable>,
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = DefaultNiche)]
            field: Option<Middle>,
        }

        assert_eq!(
            size_of::<ArchivedMiddle>(),
            2 * size_of::<ArchivedNichable>()
        );
        assert_eq!(size_of::<ArchivedOuter>(), size_of::<ArchivedMiddle>());

        let values = [
            Outer { field: None },
            Outer {
                field: Some(Middle { a: None, b: None }),
            },
            Outer {
                field: Some(Middle {
                    a: None,
                    b: Some(Nichable::create()),
                }),
            },
        ];

        roundtrip_with(&values[0], |_, archived| {
            assert!(archived.field.is_none());
        });
        roundtrip_with(&values[1], |_, archived| {
            let middle = archived.field.as_ref().unwrap();
            assert!(middle.a.is_none());
            assert!(middle.b.is_none());
        });
        roundtrip_with(&values[2], |_, archived| {
            let middle = archived.field.as_ref().unwrap();
            assert!(middle.a.is_none());
            let b = middle.b.as_ref().unwrap();
            assert_eq!(b.not_nan, 123.456);
            assert_eq!(b.int.get(), 789);
            assert_eq!(b.boolean, true);
        });
    }

    #[test]
    fn with_enum() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        enum Nichable {
            A(#[rkyv(niche)] bool),
            B {
                #[rkyv(niche = NaN)]
                float: f32,
            },
            C,
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Middle {
            #[rkyv(with = DefaultNiche, niche = NaN)]
            nichable: Option<Nichable>,
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = NicheInto<NaN>)]
            field: Option<Middle>,
        }

        assert_eq!(size_of::<ArchivedNichable>(), size_of::<ArchivedMiddle>());
        assert_eq!(size_of::<ArchivedOuter>(), size_of::<ArchivedMiddle>());

        let values = [
            Outer { field: None },
            Outer {
                field: Some(Middle { nichable: None }),
            },
            Outer {
                field: Some(Middle {
                    nichable: Some(Nichable::A(true)),
                }),
            },
            Outer {
                field: Some(Middle {
                    nichable: Some(Nichable::B { float: f32::NAN }),
                }),
            },
            Outer {
                field: Some(Middle {
                    nichable: Some(Nichable::B { float: 123.45 }),
                }),
            },
            Outer {
                field: Some(Middle {
                    nichable: Some(Nichable::C),
                }),
            },
        ];

        roundtrip_with(&values[0], |_, archived| {
            assert!(archived.field.is_none());
        });
        roundtrip_with(&values[1], |_, archived| {
            let middle = archived.field.as_ref().unwrap();
            assert!(middle.nichable.is_none());
        });
        roundtrip_with(&values[2], |_, archived| {
            let middle = archived.field.as_ref().unwrap();
            let nichable = middle.nichable.as_ref().unwrap();
            match nichable {
                ArchivedNichable::A(b) => assert!(*b),
                _ => panic!("expected `ArchivedNichable::A`"),
            }
        });
        to_archived(&values[3], |archived| {
            // no roundtrip; NAN will be interpreted as being niched
            assert!(archived.field.is_none());
        });
        roundtrip_with(&values[4], |_, archived| {
            let middle = archived.field.as_ref().unwrap();
            let nichable = middle.nichable.as_ref().unwrap();
            match nichable {
                ArchivedNichable::B { float } => {
                    assert_eq!(float.to_native(), 123.45)
                }
                _ => panic!("expected `ArchivedNichable::B`"),
            }
        });
        roundtrip_with(&values[5], |_, archived| {
            let middle = archived.field.as_ref().unwrap();
            let nichable = middle.nichable.as_ref().unwrap();
            match nichable {
                ArchivedNichable::C => {}
                _ => panic!("expected `ArchivedNichable::C`"),
            }
        });
    }

    #[test]
    fn map_niche() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = MapNiche<AsBox>)]
            opt: Option<NotNichable>,
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct NotNichable {
            int: i64,
        }

        let values = &[
            Outer { opt: None },
            Outer {
                opt: Some(NotNichable { int: 42 }),
            },
        ];

        to_bytes(&values[0], |bytes| {
            assert_eq!(
                bytes.len(),
                size_of::<ArchivedBox<ArchivedNotNichable>>()
            );
            to_archived_from_bytes::<Outer>(bytes, |archived| {
                assert!(archived.opt.as_ref().is_none());
                let deserialized: Outer = deserialize(&*archived);
                assert_eq!(&values[0], &deserialized);
            });
        });
        roundtrip_with(&values[1], |_, archived| {
            let bar = archived.opt.as_ref().unwrap();
            assert_eq!(bar.int.to_native(), 42);
        });
    }
}
