use crate::{
    boxed::ArchivedBox,
    niche::niching::{Niching, Null},
    traits::ArchivePointee,
    Place, Portable, RelPtr,
};

unsafe impl<T> Niching<ArchivedBox<T>> for Null
where
    T: ArchivePointee + Portable + ?Sized,
{
    type Niched = RelPtr<T>;

    fn niched_ptr(ptr: *const ArchivedBox<T>) -> *const Self::Niched {
        ptr.cast()
    }

    unsafe fn is_niched(niched: *const ArchivedBox<T>) -> bool {
        unsafe { (*Self::niched_ptr(niched)).is_invalid() }
    }

    fn resolve_niched(out: Place<ArchivedBox<T>>) {
        let out = unsafe { out.cast_unchecked::<Self::Niched>() };
        RelPtr::emplace_invalid(out);
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU32;

    use crate::{
        api::test::roundtrip_with,
        niche::niching::{NaN, Null, Zero},
        with::Nicher,
        Archive, Deserialize, Serialize,
    };

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[rkyv(crate, derive(Debug))]
    struct Nichable {
        #[rkyv(niche = NaN)]
        not_nan: f32,
        #[rkyv(niche = Zero)]
        int: NonZeroU32,
        #[rkyv(niche = Null)]
        boxed: Box<i32>,
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[rkyv(crate, derive(Debug))]
    struct Middle {
        #[rkyv(with = Nicher<Zero>, niche = NaN, niche = Null)]
        a: Option<Nichable>,
        #[rkyv(with = Nicher<NaN>, niche = Zero)]
        b: Option<Nichable>,
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[rkyv(crate, derive(Debug))]
    struct Outer {
        #[rkyv(with = Nicher<NaN>)]
        field: Option<Middle>,
    }

    #[test]
    fn with_struct() {
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
                    b: Some(Nichable {
                        not_nan: 123.456,
                        int: unsafe { NonZeroU32::new_unchecked(789) },
                        boxed: Box::new(727),
                    }),
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
            assert_eq!(*b.boxed.as_ref(), 727);
        });
    }
}
