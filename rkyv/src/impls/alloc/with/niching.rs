use core::marker::PhantomData;

use rancor::Fallible;

use crate::{
    alloc::boxed::Box,
    niche::niching::{Niching, Null},
    traits::ArchivePointee,
    Archive, ArchiveUnsized, Archived, Place, RelPtr, Serialize,
};

pub struct NichedBox<T: ?Sized>(PhantomData<T>);

impl<T: ArchivePointee + ?Sized> Archive for NichedBox<T> {
    type Archived = RelPtr<T>;
    type Resolver = ();

    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        RelPtr::emplace_invalid(out);
    }
}

impl<T, S> Serialize<S> for NichedBox<T>
where
    T: ArchivePointee + ?Sized,
    S: Fallible + ?Sized,
{
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

unsafe impl<T: ArchiveUnsized + ?Sized> Niching<Box<T>> for Null {
    type Niched = NichedBox<T::Archived>;

    fn niched() -> Self::Niched {
        NichedBox(PhantomData)
    }

    fn is_niched(niched: &Archived<Self::Niched>) -> bool {
        niched.is_invalid()
    }
}
