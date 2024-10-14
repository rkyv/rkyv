use crate::{
    boxed::ArchivedBox,
    niche::niching::{DefaultNicher, Niching, Null},
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

unsafe impl<T> Niching<ArchivedBox<T>> for DefaultNicher
where
    T: ArchivePointee + Portable + ?Sized,
{
    type Niched = <Null as Niching<ArchivedBox<T>>>::Niched;

    fn niched_ptr(ptr: *const ArchivedBox<T>) -> *const Self::Niched {
        <Null as Niching<ArchivedBox<T>>>::niched_ptr(ptr)
    }

    unsafe fn is_niched(niched: *const ArchivedBox<T>) -> bool {
        unsafe { <Null as Niching<ArchivedBox<T>>>::is_niched(niched) }
    }

    fn resolve_niched(out: Place<ArchivedBox<T>>) {
        <Null as Niching<ArchivedBox<T>>>::resolve_niched(out);
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        api::test::roundtrip_with, niche::niching::DefaultNicher, Archive,
        Deserialize, Serialize,
    };

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[rkyv(crate, derive(Debug))]
    struct Nichable {
        #[rkyv(niche)] // Default = Null
        boxed: Box<i32>,
    }

    #[test]
    fn with_struct() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, derive(Debug))]
        struct Outer {
            #[rkyv(with = DefaultNicher)]
            field: Option<Nichable>,
        }

        assert_eq!(size_of::<ArchivedNichable>(), size_of::<ArchivedOuter>());

        let values = [
            Outer { field: None },
            Outer {
                field: Some(Nichable {
                    boxed: Box::new(727),
                }),
            },
        ];

        roundtrip_with(&values[0], |_, archived| {
            assert!(archived.field.is_none());
        });
        roundtrip_with(&values[1], |_, archived| {
            let nichable = archived.field.as_ref().unwrap();
            assert_eq!(nichable.boxed.as_ref().to_native(), 727);
        });
    }
}
