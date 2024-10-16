use core::num::{NonZeroI8, NonZeroU8};

use crate::{
    niche::{
        niched_option::NichedOption,
        niching::{Bool, DefaultNicher, NaN, Niching, SharedNiching, Zero},
    },
    primitive::{
        ArchivedF32, ArchivedF64, ArchivedNonZeroI128, ArchivedNonZeroI16,
        ArchivedNonZeroI32, ArchivedNonZeroI64, ArchivedNonZeroU128,
        ArchivedNonZeroU16, ArchivedNonZeroU32, ArchivedNonZeroU64,
    },
    Archived, Place,
};

// Zero

macro_rules! impl_nonzero_zero_niching {
    ($nz:ty, $ar:ty) => {
        unsafe impl Niching<$nz> for Zero {
            type Niched = Archived<$ar>;

            unsafe fn niched_ptr(
                ptr: *const $nz,
            ) -> Option<*const Self::Niched> {
                Some(ptr.cast())
            }

            unsafe fn is_niched(niched: *const $nz) -> bool {
                unsafe { *niched.cast::<Self::Niched>() == 0 }
            }

            fn resolve_niched(out: Place<$nz>) {
                unsafe { out.cast_unchecked::<Self::Niched>() }.write(0.into());
            }
        }

        unsafe impl Niching<$nz> for DefaultNicher {
            type Niched = <Zero as Niching<$nz>>::Niched;

            unsafe fn niched_ptr(
                ptr: *const $nz,
            ) -> Option<*const Self::Niched> {
                unsafe { <Zero as Niching<$nz>>::niched_ptr(ptr) }
            }

            unsafe fn is_niched(niched: *const $nz) -> bool {
                unsafe { <Zero as Niching<$nz>>::is_niched(niched) }
            }

            fn resolve_niched(out: Place<$nz>) {
                <Zero as Niching<$nz>>::resolve_niched(out);
            }
        }
    };
}

impl_nonzero_zero_niching!(NonZeroU8, u8);
impl_nonzero_zero_niching!(ArchivedNonZeroU16, u16);
impl_nonzero_zero_niching!(ArchivedNonZeroU32, u32);
impl_nonzero_zero_niching!(ArchivedNonZeroU64, u64);
impl_nonzero_zero_niching!(ArchivedNonZeroU128, u128);

impl_nonzero_zero_niching!(NonZeroI8, i8);
impl_nonzero_zero_niching!(ArchivedNonZeroI16, i16);
impl_nonzero_zero_niching!(ArchivedNonZeroI32, i32);
impl_nonzero_zero_niching!(ArchivedNonZeroI64, i64);
impl_nonzero_zero_niching!(ArchivedNonZeroI128, i128);

// NaN

macro_rules! impl_float_nan_niching {
    ($fl:ty, $ar:ty) => {
        unsafe impl Niching<$ar> for NaN {
            type Niched = $ar;

            unsafe fn niched_ptr(
                ptr: *const $ar,
            ) -> Option<*const Self::Niched> {
                Some(ptr)
            }

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

unsafe impl Niching<bool> for Bool {
    type Niched = u8;

    unsafe fn niched_ptr(ptr: *const bool) -> Option<*const Self::Niched> {
        Some(ptr.cast())
    }

    unsafe fn is_niched(niched: *const bool) -> bool {
        unsafe { (*niched.cast::<Self::Niched>()) > 1 }
    }

    fn resolve_niched(out: Place<bool>) {
        unsafe { out.cast_unchecked::<Self::Niched>().write(2) };
    }
}

unsafe impl Niching<bool> for DefaultNicher {
    type Niched = <Bool as Niching<bool>>::Niched;

    unsafe fn niched_ptr(ptr: *const bool) -> Option<*const Self::Niched> {
        unsafe { <Bool as Niching<bool>>::niched_ptr(ptr) }
    }

    unsafe fn is_niched(niched: *const bool) -> bool {
        unsafe { <Bool as Niching<bool>>::is_niched(niched) }
    }

    fn resolve_niched(out: Place<bool>) {
        <Bool as Niching<bool>>::resolve_niched(out);
    }
}

// -------

unsafe impl<T, N1, N2> Niching<NichedOption<T, N1>> for N2
where
    T: SharedNiching<N1, N2>,
    N1: Niching<T>,
    N2: Niching<T>,
{
    type Niched = <Self as Niching<T>>::Niched;

    unsafe fn niched_ptr(
        ptr: *const NichedOption<T, N1>,
    ) -> Option<*const Self::Niched> {
        unsafe { <Self as Niching<T>>::niched_ptr(ptr.cast()) }
    }

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
        api::test::roundtrip_with,
        niche::niching::{DefaultNicher, NaN, Zero},
        with::Nicher,
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
            #[rkyv(with = Nicher<Zero>, niche = NaN, niche)] // Default = Bool
            a: Option<Nichable>,
            #[rkyv(with = Nicher<NaN>, niche = Zero)]
            b: Option<Nichable>,
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = DefaultNicher)]
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
            B(u8),
        }

        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = DefaultNicher)]
            field: Option<Nichable>,
        }

        let tag_size = size_of::<u8>().max(align_of::<ArchivedNichable>());
        assert_eq!(
            size_of::<ArchivedNichable>(),
            tag_size + size_of::<bool>().max(size_of::<u8>())
        );
        assert_eq!(size_of::<ArchivedOuter>(), size_of::<ArchivedNichable>());

        let values = [
            Outer { field: None },
            Outer {
                field: Some(Nichable::A(true)),
            },
            Outer {
                field: Some(Nichable::B(0)),
            },
            Outer {
                field: Some(Nichable::B(2)),
            },
        ];

        roundtrip_with(&values[0], |_, archived| {
            assert!(archived.field.is_none());
        });
        roundtrip_with(&values[1], |_, archived| {
            let nichable = archived.field.as_ref().unwrap();
            match nichable {
                ArchivedNichable::A(b) => assert!(*b),
                _ => panic!("expected `ArchivedNichable::A`"),
            }
        });
        roundtrip_with(&values[2], |_, archived| {
            let nichable = archived.field.as_ref().unwrap();
            match nichable {
                ArchivedNichable::B(n) => assert_eq!(*n, 0),
                _ => panic!("expected `ArchivedNichable::B`"),
            }
        });
        roundtrip_with(&values[3], |_, archived| {
            let nichable = archived.field.as_ref().unwrap();
            match nichable {
                ArchivedNichable::B(n) => assert_eq!(*n, 2),
                _ => panic!("expected `ArchivedNichable::B`"),
            }
        });
    }
}
