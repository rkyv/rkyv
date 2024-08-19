//! Archived versions of shared pointers.

use core::{
    borrow::Borrow, cmp, fmt, hash, marker::PhantomData, ops::Deref, pin::Pin,
};

use munge::munge;
use rancor::Fallible;

use crate::{
    place::Initialized,
    ser::{Sharing, SharingExt, Writer, WriterExt as _},
    traits::{ArchivePointee, Freeze},
    ArchiveUnsized, Place, Portable, RelPtr, SerializeUnsized,
};

/// The flavor type for [`Rc`](std::rc::Rc).
pub struct RcFlavor;

/// The flavor type for [`Arc`](std::sync::Arc).
pub struct ArcFlavor;

/// An archived `Rc`.
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type paired with
/// a "flavor" type. Because there may be many varieties of shared pointers and
/// they may not be used together, the flavor helps check that memory is not
/// being shared incorrectly during validation.
#[derive(Freeze, Portable)]
#[rkyv(crate)]
#[repr(transparent)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    check_bytes(verify)
)]
pub struct ArchivedRc<T: ArchivePointee + ?Sized, F> {
    ptr: RelPtr<T>,
    _phantom: PhantomData<F>,
}

impl<T: ArchivePointee + ?Sized, F> ArchivedRc<T, F> {
    /// Gets the value of the `ArchivedRc`.
    pub fn get(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }

    /// Gets the pinned mutable value of this `ArchivedRc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` pointers to the same value must not be
    /// dereferenced for the duration of the returned borrow.
    pub unsafe fn get_pin_unchecked(self: Pin<&mut Self>) -> Pin<&mut T> {
        let ptr = unsafe { self.map_unchecked_mut(|s| &mut s.ptr) };
        unsafe { Pin::new_unchecked(&mut *ptr.as_mut_ptr()) }
    }

    /// Resolves an archived `Rc` from a given reference.
    pub fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        resolver: RcResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedRc { ptr, .. } = out);
        RelPtr::emplace_unsized(resolver.pos, value.archived_metadata(), ptr);
    }

    /// Serializes an archived `Rc` from a given reference.
    pub fn serialize_from_ref<U, S>(
        value: &U,
        serializer: &mut S,
    ) -> Result<RcResolver, S::Error>
    where
        U: SerializeUnsized<S> + ?Sized,
        S: Fallible + Writer + Sharing + ?Sized,
    {
        let pos = serializer.serialize_shared(value)?;

        // The positions of serialized `Rc` values must be unique. If we didn't
        // write any data by serializing `value`, pad the serializer by a byte
        // to ensure that our position will be unique.
        if serializer.pos() == pos {
            serializer.pad(1)?;
        }

        Ok(RcResolver { pos })
    }
}

impl<T: ArchivePointee + ?Sized, F> AsRef<T> for ArchivedRc<T, F> {
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized, F> Borrow<T> for ArchivedRc<T, F> {
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Debug + ?Sized, F> fmt::Debug
    for ArchivedRc<T, F>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + ?Sized, F> Deref for ArchivedRc<T, F> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Display + ?Sized, F> fmt::Display
    for ArchivedRc<T, F>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + Eq + ?Sized, F> Eq for ArchivedRc<T, F> {}

impl<T: ArchivePointee + hash::Hash + ?Sized, F> hash::Hash
    for ArchivedRc<T, F>
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<T: ArchivePointee + Ord + ?Sized, F> Ord for ArchivedRc<T, F> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.get().cmp(other.get())
    }
}

impl<T, TF, U, UF> PartialEq<ArchivedRc<U, UF>> for ArchivedRc<T, TF>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ArchivePointee + ?Sized,
{
    fn eq(&self, other: &ArchivedRc<U, UF>) -> bool {
        self.get().eq(other.get())
    }
}

impl<T, TF, U, UF> PartialOrd<ArchivedRc<U, UF>> for ArchivedRc<T, TF>
where
    T: ArchivePointee + PartialOrd<U> + ?Sized,
    U: ArchivePointee + ?Sized,
{
    fn partial_cmp(&self, other: &ArchivedRc<U, UF>) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T, F> fmt::Pointer for ArchivedRc<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Pointer::fmt(&self.ptr.base(), f)
    }
}

/// The resolver for `Rc`.
pub struct RcResolver {
    pos: usize,
}

/// An archived `rc::Weak`.
///
/// This is essentially just an optional [`ArchivedRc`].
#[derive(Freeze, Portable)]
#[rkyv(crate)]
#[repr(u8)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
pub enum ArchivedRcWeak<T: ArchivePointee + ?Sized, F> {
    /// A null weak pointer
    None,
    /// A weak pointer to some shared pointer
    Some(ArchivedRc<T, F>),
}

impl<T: ArchivePointee + ?Sized, F> ArchivedRcWeak<T, F> {
    /// Attempts to upgrade the weak pointer to an `ArchivedArc`.
    ///
    /// Returns `None` if a null weak pointer was serialized.
    pub fn upgrade(&self) -> Option<&ArchivedRc<T, F>> {
        match self {
            ArchivedRcWeak::None => None,
            ArchivedRcWeak::Some(r) => Some(r),
        }
    }

    /// Attempts to upgrade a pinned mutable weak pointer.
    pub fn upgrade_pin(
        self: Pin<&mut Self>,
    ) -> Option<Pin<&mut ArchivedRc<T, F>>> {
        unsafe {
            match self.get_unchecked_mut() {
                ArchivedRcWeak::None => None,
                ArchivedRcWeak::Some(r) => Some(Pin::new_unchecked(r)),
            }
        }
    }

    /// Resolves an archived `Weak` from a given optional reference.
    pub fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: Option<&U>,
        resolver: RcWeakResolver,
        out: Place<Self>,
    ) {
        match resolver {
            RcWeakResolver::None => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedRcWeakVariantNone>()
                };
                munge!(let ArchivedRcWeakVariantNone(tag) = out);
                tag.write(ArchivedRcWeakTag::None);
            }
            RcWeakResolver::Some(resolver) => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedRcWeakVariantSome<T, F>>()
                };
                munge!(let ArchivedRcWeakVariantSome(tag, rc) = out);
                tag.write(ArchivedRcWeakTag::Some);

                ArchivedRc::resolve_from_ref(value.unwrap(), resolver, rc);
            }
        }
    }

    /// Serializes an archived `Weak` from a given optional reference.
    pub fn serialize_from_ref<U, S>(
        value: Option<&U>,
        serializer: &mut S,
    ) -> Result<RcWeakResolver, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: Fallible + Writer + Sharing + ?Sized,
    {
        Ok(match value {
            None => RcWeakResolver::None,
            Some(r) => RcWeakResolver::Some(
                ArchivedRc::<T, F>::serialize_from_ref(r, serializer)?,
            ),
        })
    }
}

impl<T: ArchivePointee + fmt::Debug + ?Sized, F> fmt::Debug
    for ArchivedRcWeak<T, F>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Weak)")
    }
}

/// The resolver for `rc::Weak`.
pub enum RcWeakResolver {
    /// The weak pointer was null
    None,
    /// The weak pointer was to some shared pointer
    Some(RcResolver),
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedRcWeakTag {
    None,
    Some,
}

// SAFETY: `ArchivedRcWeakTag` is `repr(u8)` and so is always initialized.
unsafe impl Initialized for ArchivedRcWeakTag {}

#[repr(C)]
struct ArchivedRcWeakVariantNone(ArchivedRcWeakTag);

#[repr(C)]
struct ArchivedRcWeakVariantSome<T: ArchivePointee + ?Sized, F>(
    ArchivedRcWeakTag,
    ArchivedRc<T, F>,
);

#[cfg(feature = "bytecheck")]
mod verify {
    use core::any::TypeId;

    use bytecheck::{
        rancor::{Fallible, Source},
        CheckBytes, Verify,
    };

    use super::ArchivedRc;
    use crate::{
        traits::{ArchivePointee, LayoutRaw},
        validation::{ArchiveContext, ArchiveContextExt, SharedContext},
    };

    unsafe impl<T, F, C> Verify<C> for ArchivedRc<T, F>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized + 'static,
        T::ArchivedMetadata: CheckBytes<C>,
        F: 'static,
        C: Fallible + ArchiveContext + SharedContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr = self.ptr.as_ptr_wrapping();
            let type_id = TypeId::of::<ArchivedRc<T, F>>();

            let is_new = context
                .register_shared_ptr(ptr as *const u8 as usize, type_id)?;
            if is_new {
                context.in_subtree(ptr, |context| unsafe {
                    T::check_bytes(ptr, context)
                })?
            }

            Ok(())
        }
    }
}
