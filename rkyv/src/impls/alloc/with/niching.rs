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

    fn is_niched(niched: *const ArchivedBox<T>) -> bool {
        unsafe { (*Self::niched_ptr(niched)).is_invalid() }
    }

    fn resolve_niched(out: Place<ArchivedBox<T>>) {
        let out = unsafe { out.cast_unchecked::<Self::Niched>() };
        RelPtr::emplace_invalid(out);
    }
}
