use core::mem::ManuallyDrop;

use rkyv_derive::Portable;

use crate::{
    boxed::{ArchivedBox, BoxResolver},
    niche::decider::{Decider, Null},
    traits::ArchivePointee,
    Archive, ArchiveUnsized, Archived, Place, RelPtr,
};

#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
pub union NullNichedBox<T: ArchivePointee + ?Sized> {
    boxed: ManuallyDrop<ArchivedBox<T>>,
    ptr: ManuallyDrop<RelPtr<T>>,
}

impl<T: ArchivePointee + ?Sized> NullNichedBox<T> {
    fn is_invalid(&self) -> bool {
        unsafe { self.ptr.is_invalid() }
    }
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use rancor::Fallible;

    use crate::{
        bytecheck::{CheckBytes, Verify},
        rancor::Source,
        traits::LayoutRaw,
        validation::ArchiveContext,
    };

    unsafe impl<T, C> CheckBytes<C> for NullNichedBox<T>
    where
        T: ArchivePointee + ?Sized,
        C: Fallible + ?Sized,
        RelPtr<T>: CheckBytes<C>,
        Self: Verify<C>,
    {
        unsafe fn check_bytes(
            value: *const Self,
            context: &mut C,
        ) -> Result<(), C::Error> {
            // SAFETY: `Repr<T>` is a `#[repr(C)]` union of an `ArchivedBox<T>`
            // and a `RelPtr<T>`, and so is guaranteed to be aligned and point
            // to enough bytes for a `RelPtr<T>`.
            unsafe {
                RelPtr::check_bytes(value.cast::<RelPtr<T>>(), context)?;
            }

            // verify with null check
            Self::verify(unsafe { &*value }, context)
        }
    }

    unsafe impl<T, C> Verify<C> for NullNichedBox<T>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized,
        T::ArchivedMetadata: CheckBytes<C>,
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let is_invalid = unsafe { self.ptr.is_invalid() };
            if is_invalid {
                // This is niched and doesn't need to be checked further
                Ok(())
            } else {
                unsafe { self.boxed.verify(context) }
            }
        }
    }
};

impl<T> Decider<Box<T>> for Null
where
    T: ArchiveUnsized + ?Sized,
{
    type Archived = NullNichedBox<T::Archived>;

    fn as_option(archived: &Self::Archived) -> Option<&Archived<Box<T>>> {
        if archived.is_invalid() {
            None
        } else {
            unsafe { Some(&archived.boxed) }
        }
    }

    fn resolve_from_option(
        option: Option<&Box<T>>,
        resolver: Option<BoxResolver>,
        out: Place<Self::Archived>,
    ) {
        match option {
            Some(value) => {
                let resolver = resolver.expect("non-niched resolver");
                let out = unsafe { out.cast_unchecked::<Archived<Box<T>>>() };
                value.resolve(resolver, out);
            }
            None => {
                let out =
                    unsafe { out.cast_unchecked::<RelPtr<T::Archived>>() };
                RelPtr::emplace_invalid(out);
            }
        }
    }
}
