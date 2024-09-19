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

    fn is_niched(niched: &Self::Niched) -> bool {
        niched.is_invalid()
    }

    fn resolve_niche(out: Place<Self::Niched>) {
        RelPtr::emplace_invalid(out);
    }
}
