use crate::{
    alloc::boxed::Box,
    niche::decider::{Decider, Null},
    ArchiveUnsized, Place, RelPtr,
};

unsafe impl<T> Decider<Box<T>> for Null
where
    T: ArchiveUnsized + ?Sized,
{
    type Niched = RelPtr<T::Archived>;

    fn is_niched(niched: &Self::Niched) -> bool {
        niched.is_invalid()
    }

    fn resolve_niche(out: Place<Self::Niched>) {
        RelPtr::emplace_invalid(out);
    }
}
