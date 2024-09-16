//! An archived version of `Box`.

use core::{borrow::Borrow, cmp, fmt, hash, ops::Deref};

use munge::munge;
use rancor::Fallible;

use crate::{
    primitive::FixedUsize, seal::Seal, traits::ArchivePointee, ArchiveUnsized,
    Place, Portable, RelPtr, SerializeUnsized,
};

/// An archived [`Box`].
///
/// This is a thin `#[repr(transparent)]` wrapper around a [`RelPtr`] to the
/// archived type.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[repr(transparent)]
pub struct ArchivedBox<T: ArchivePointee + ?Sized> {
    ptr: RelPtr<T>,
}

impl<T: ArchivePointee + ?Sized> ArchivedBox<T> {
    /// Returns a reference to the value of this archived box.
    pub fn get(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }

    /// Returns a sealed mutable reference to the value of this archived box.
    pub fn get_seal(this: Seal<'_, Self>) -> Seal<'_, T> {
        munge!(let Self { ptr } = this);
        Seal::new(unsafe { &mut *RelPtr::as_mut_ptr(ptr) })
    }

    /// Resolves an archived box from the given value and parameters.
    pub fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        resolver: BoxResolver,
        out: Place<Self>,
    ) {
        Self::resolve_from_raw_parts(resolver, value.archived_metadata(), out)
    }

    /// Serializes an archived box from the given value and serializer.
    pub fn serialize_from_ref<U, S>(
        value: &U,
        serializer: &mut S,
    ) -> Result<BoxResolver, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: Fallible + ?Sized,
    {
        Ok(BoxResolver {
            pos: value.serialize_unsized(serializer)? as FixedUsize,
        })
    }

    /// Resolves an archived box from a [`BoxResolver`] and the raw metadata
    /// directly.
    pub fn resolve_from_raw_parts(
        resolver: BoxResolver,
        metadata: T::ArchivedMetadata,
        out: Place<Self>,
    ) {
        munge!(let ArchivedBox { ptr } = out);
        RelPtr::emplace_unsized(resolver.pos as usize, metadata, ptr);
    }
}

impl<T: ArchivePointee + ?Sized> AsRef<T> for ArchivedBox<T> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> Borrow<T> for ArchivedBox<T> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Debug for ArchivedBox<T>
where
    T::ArchivedMetadata: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArchivedBox").field(&self.ptr).finish()
    }
}

impl<T: ArchivePointee + ?Sized> Deref for ArchivedBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Display + ?Sized> fmt::Display
    for ArchivedBox<T>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + Eq + ?Sized> Eq for ArchivedBox<T> {}

impl<T: ArchivePointee + hash::Hash + ?Sized> hash::Hash for ArchivedBox<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: ArchivePointee + Ord + ?Sized> Ord for ArchivedBox<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ArchivePointee + ?Sized>
    PartialEq<ArchivedBox<U>> for ArchivedBox<T>
{
    fn eq(&self, other: &ArchivedBox<U>) -> bool {
        self.get().eq(other.get())
    }
}

impl<T: ArchivePointee + PartialOrd + ?Sized> PartialOrd for ArchivedBox<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Pointer for ArchivedBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.get() as *const T;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// The resolver for `Box`.
pub struct BoxResolver {
    pos: FixedUsize,
}

impl BoxResolver {
    /// Creates a new [`BoxResolver`] from the position of a serialized value.
    ///
    /// In most cases, you won't need to create a [`BoxResolver`] yourself and
    /// can instead obtain it through [`ArchivedBox::serialize_from_ref`].
    pub fn from_pos(pos: usize) -> Self {
        Self {
            pos: pos as FixedUsize,
        }
    }
}

#[cfg(feature = "bytecheck")]
mod verify {
    use bytecheck::{
        rancor::{Fallible, Source},
        CheckBytes, Verify,
    };

    use crate::{
        boxed::ArchivedBox,
        traits::{ArchivePointee, LayoutRaw},
        validation::{ArchiveContext, ArchiveContextExt},
    };

    unsafe impl<T, C> Verify<C> for ArchivedBox<T>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized,
        T::ArchivedMetadata: CheckBytes<C>,
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr = self.ptr.as_ptr_wrapping();
            context.in_subtree(ptr, |context| unsafe {
                T::check_bytes(ptr, context)
            })
        }
    }
}
