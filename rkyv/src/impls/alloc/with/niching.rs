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

    fn is_niched(niched: *const ArchivedBox<T>) -> bool {
        unsafe { (*niched.cast::<Self::Niched>()).is_invalid() }
    }

    fn resolve_niched(out: *mut ArchivedBox<T>) {
        let out = unsafe { Place::new_unchecked(0, out.cast::<RelPtr<T>>()) };
        RelPtr::emplace_invalid(out);
    }

    #[cfg(feature = "bytecheck")]
    unsafe fn checked_is_niched<C>(
        niched: *const ArchivedBox<T>,
        context: &mut C,
    ) -> Result<bool, C::Error>
    where
        C: rancor::Fallible + ?Sized,
        Self::Niched: bytecheck::CheckBytes<C>,
    {
        unsafe {
            <RelPtr<T> as bytecheck::CheckBytes<C>>::check_bytes(
                niched.cast::<Self::Niched>(),
                context,
            )?
        };

        Ok(Self::is_niched(niched))
    }
}
