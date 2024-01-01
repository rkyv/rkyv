//! Archived versions of shared pointers.

use crate::{
    ser::{
        Serializer, SerializerExt as _, SharedSerializeRegistry,
        SharedSerializeRegistryExt,
    },
    ArchivePointee, ArchiveUnsized, RelPtr, SerializeUnsized,
};
use core::{
    borrow::Borrow, cmp, fmt, hash, marker::PhantomData, ops::Deref, pin::Pin,
    ptr,
};
use rancor::Fallible;

/// The flavor type for [`Rc`](std::rc::Rc).
pub struct RcFlavor;

/// The flavor type for [`Arc`](std::sync::Arc).
pub struct ArcFlavor;

/// An archived `Rc`.
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type paired with a "flavor" type.
/// Because there may be many varieties of shared pointers and they may not be used together, the
/// flavor helps check that memory is not being shared incorrectly during validation.
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
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }

    /// Gets the pinned mutable value of this `ArchivedRc`.
    ///
    /// # Safety
    ///
    /// Any other `ArchivedRc` pointers to the same value must not be dereferenced for the duration
    /// of the returned borrow.
    #[inline]
    pub unsafe fn get_pin_mut_unchecked(self: Pin<&mut Self>) -> Pin<&mut T> {
        self.map_unchecked_mut(|s| &mut *s.ptr.as_ptr())
    }

    /// Resolves an archived `Rc` from a given reference.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        pos: usize,
        resolver: RcResolver,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.ptr);
        RelPtr::emplace_unsized(
            pos + fp,
            resolver.pos,
            value.archived_metadata(),
            fo,
        );
    }

    /// Serializes an archived `Rc` from a given reference.
    #[inline]
    pub fn serialize_from_ref<U, S>(
        value: &U,
        serializer: &mut S,
    ) -> Result<RcResolver, S::Error>
    where
        U: SerializeUnsized<S> + ?Sized,
        S: Fallible + Serializer + SharedSerializeRegistry + ?Sized,
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
    #[inline]
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized, F> Borrow<T> for ArchivedRc<T, F> {
    #[inline]
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Debug + ?Sized, F> fmt::Debug
    for ArchivedRc<T, F>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + ?Sized, F> Deref for ArchivedRc<T, F> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Display + ?Sized, F> fmt::Display
    for ArchivedRc<T, F>
{
    #[inline]
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
    #[inline]
    pub fn upgrade(&self) -> Option<&ArchivedRc<T, F>> {
        match self {
            ArchivedRcWeak::None => None,
            ArchivedRcWeak::Some(r) => Some(r),
        }
    }

    /// Attempts to upgrade a pinned mutable weak pointer.
    #[inline]
    pub fn upgrade_pin_mut(
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
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: Option<&U>,
        pos: usize,
        resolver: RcWeakResolver,
        out: *mut Self,
    ) {
        match resolver {
            RcWeakResolver::None => {
                let out = out.cast::<ArchivedRcWeakVariantNone>();
                ptr::addr_of_mut!((*out).0).write(ArchivedRcWeakTag::None);
            }
            RcWeakResolver::Some(resolver) => {
                let out = out.cast::<ArchivedRcWeakVariantSome<T, F>>();
                ptr::addr_of_mut!((*out).0).write(ArchivedRcWeakTag::Some);

                let (fp, fo) = out_field!(out.1);
                ArchivedRc::resolve_from_ref(
                    value.unwrap(),
                    pos + fp,
                    resolver,
                    fo,
                );
            }
        }
    }

    /// Serializes an archived `Weak` from a given optional reference.
    #[inline]
    pub fn serialize_from_ref<U, S>(
        value: Option<&U>,
        serializer: &mut S,
    ) -> Result<RcWeakResolver, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: Fallible + Serializer + SharedSerializeRegistry + ?Sized,
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

#[repr(C)]
struct ArchivedRcWeakVariantNone(ArchivedRcWeakTag);

#[repr(C)]
struct ArchivedRcWeakVariantSome<T: ArchivePointee + ?Sized, F>(
    ArchivedRcWeakTag,
    ArchivedRc<T, F>,
);

#[cfg(feature = "bytecheck")]
mod verify {
    use super::ArchivedRc;
    use crate::{
        validation::{
            ArchiveContext, ArchiveContextExt, LayoutRaw, SharedContext,
        },
        ArchivePointee,
    };
    use bytecheck::{
        rancor::{Error, Fallible},
        CheckBytes, Verify,
    };
    use core::any::TypeId;

    unsafe impl<T, F, C> Verify<C> for ArchivedRc<T, F>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized + 'static,
        T::ArchivedMetadata: CheckBytes<C>,
        F: 'static,
        C: Fallible + ArchiveContext + SharedContext + ?Sized,
        C::Error: Error,
    {
        #[inline]
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr = self.ptr.as_ptr_wrapping();
            let type_id = TypeId::of::<ArchivedRc<T, F>>();

            if context
                .register_shared_ptr(ptr as *const u8 as usize, type_id)?
            {
                unsafe {
                    context.bounds_check_subtree_rel_ptr(&self.ptr)?;
                }

                let range = unsafe { context.push_prefix_subtree(ptr)? };
                unsafe {
                    T::check_bytes(ptr, context)?;
                }
                unsafe {
                    context.pop_subtree_range(range)?;
                }
            }
            Ok(())
        }
    }
}
