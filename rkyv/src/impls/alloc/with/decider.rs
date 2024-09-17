use crate::{
    alloc::boxed::Box,
    niche::{
        decider::{Decider, Null},
        niched_option::NichedOption,
    },
    ArchiveUnsized, Place, RelPtr,
};

unsafe impl<T> Decider<Box<T>> for Null
where
    T: ArchiveUnsized + ?Sized,
{
    type Niched = RelPtr<T::Archived>;

    fn is_none(option: &NichedOption<Box<T>, Self>) -> bool {
        unsafe { &option.niche }.is_invalid()
    }

    fn resolve_niche(out: Place<Self::Niched>) {
        RelPtr::emplace_invalid(out);
    }
}
