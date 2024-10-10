//! Archived versions of shared pointers.

use core::{borrow::Borrow, cmp, fmt, hash, marker::PhantomData, ops::Deref};

use munge::munge;
use rancor::{Fallible, Source};

use crate::{
    primitive::FixedUsize,
    seal::Seal,
    ser::{Sharing, SharingExt, Writer, WriterExt as _},
    traits::ArchivePointee,
    ArchiveUnsized, Place, Portable, RelPtr, SerializeUnsized,
};

/// A type marker for `ArchivedRc`.
pub trait Flavor: 'static {
    /// If `true`, cyclic `ArchivedRc`s with this flavor will not fail
    /// validation. If `false`, cyclic `ArchivedRc`s with this flavor will fail
    /// validation.
    const ALLOW_CYCLES: bool;
}

/// The flavor type for [`Rc`](crate::alloc::rc::Rc).
pub struct RcFlavor;

impl Flavor for RcFlavor {
    const ALLOW_CYCLES: bool = false;
}

/// The flavor type for [`Arc`](crate::alloc::sync::Arc).
pub struct ArcFlavor;

impl Flavor for ArcFlavor {
    const ALLOW_CYCLES: bool = false;
}

/// An archived `Rc`.
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type paired with
/// a "flavor" type. Because there may be many varieties of shared pointers and
/// they may not be used together, the flavor helps check that memory is not
/// being shared incorrectly during validation.
#[derive(Portable)]
#[rkyv(crate)]
#[repr(transparent)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
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

    /// Gets the sealed value of this `ArchivedRc`.
    ///
    /// # Safety
    ///
    /// Any other pointers to the same value must not be dereferenced for the
    /// duration of the returned borrow.
    pub unsafe fn get_seal_unchecked(this: Seal<'_, Self>) -> Seal<'_, T> {
        munge!(let Self { ptr, _phantom: _ } = this);
        Seal::new(unsafe { &mut *RelPtr::as_mut_ptr(ptr) })
    }

    /// Resolves an archived `Rc` from a given reference.
    pub fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        resolver: RcResolver,
        out: Place<Self>,
    ) {
        munge!(let ArchivedRc { ptr, .. } = out);
        RelPtr::emplace_unsized(
            resolver.pos as usize,
            value.archived_metadata(),
            ptr,
        );
    }

    /// Serializes an archived `Rc` from a given reference.
    pub fn serialize_from_ref<U, S>(
        value: &U,
        serializer: &mut S,
    ) -> Result<RcResolver, S::Error>
    where
        U: SerializeUnsized<S> + ?Sized,
        S: Fallible + Writer + Sharing + ?Sized,
        S::Error: Source,
    {
        let pos = serializer.serialize_shared(value)?;

        // The positions of serialized `Rc` values must be unique. If we didn't
        // write any data by serializing `value`, pad the serializer by a byte
        // to ensure that our position will be unique.
        if serializer.pos() == pos {
            serializer.pad(1)?;
        }

        Ok(RcResolver {
            pos: pos as FixedUsize,
        })
    }
}

impl<T, F> AsRef<T> for ArchivedRc<T, F>
where
    T: ArchivePointee + ?Sized,
{
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T, F> Borrow<T> for ArchivedRc<T, F>
where
    T: ArchivePointee + ?Sized,
{
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T, F> fmt::Debug for ArchivedRc<T, F>
where
    T: ArchivePointee + fmt::Debug + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T, F> Deref for ArchivedRc<T, F>
where
    T: ArchivePointee + ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T, F> fmt::Display for ArchivedRc<T, F>
where
    T: ArchivePointee + fmt::Display + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T, F> Eq for ArchivedRc<T, F> where T: ArchivePointee + Eq + ?Sized {}

impl<T, F> hash::Hash for ArchivedRc<T, F>
where
    T: ArchivePointee + hash::Hash + ?Sized,
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state)
    }
}

impl<T, F> Ord for ArchivedRc<T, F>
where
    T: ArchivePointee + Ord + ?Sized,
{
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
    pos: FixedUsize,
}

impl RcResolver {
    /// Creates a new [`RcResolver`] from the position of a serialized value.
    ///
    /// In most cases, you won't need to create a [`RcResolver`] yourself and
    /// can instead obtain it through [`ArchivedRc::serialize_from_ref`].
    pub fn from_pos(pos: usize) -> Self {
        Self {
            pos: pos as FixedUsize,
        }
    }
}

/// An archived `rc::Weak`.
///
/// This is essentially just an optional [`ArchivedRc`].
#[derive(Portable)]
#[rkyv(crate)]
#[repr(transparent)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    bytecheck(verify)
)]
pub struct ArchivedRcWeak<T: ArchivePointee + ?Sized, F> {
    ptr: RelPtr<T>,
    _phantom: PhantomData<F>,
}

impl<T: ArchivePointee + ?Sized, F> ArchivedRcWeak<T, F> {
    /// Attempts to upgrade the weak pointer to an `ArchivedArc`.
    ///
    /// Returns `None` if a null weak pointer was serialized.
    pub fn upgrade(&self) -> Option<&ArchivedRc<T, F>> {
        if self.ptr.is_invalid() {
            None
        } else {
            Some(unsafe { &*(self as *const Self).cast() })
        }
    }

    /// Attempts to upgrade a sealed weak pointer.
    pub fn upgrade_seal(
        this: Seal<'_, Self>,
    ) -> Option<Seal<'_, ArchivedRc<T, F>>> {
        let this = unsafe { this.unseal_unchecked() };
        if this.ptr.is_invalid() {
            None
        } else {
            Some(Seal::new(unsafe { &mut *(this as *mut Self).cast() }))
        }
    }

    /// Resolves an archived `Weak` from a given optional reference.
    pub fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: Option<&U>,
        resolver: RcWeakResolver,
        out: Place<Self>,
    ) {
        match value {
            None => {
                munge!(let ArchivedRcWeak { ptr, _phantom: _ } = out);
                RelPtr::emplace_invalid(ptr);
            }
            Some(value) => {
                let out = unsafe { out.cast_unchecked::<ArchivedRc<T, F>>() };
                ArchivedRc::resolve_from_ref(value, resolver.inner, out);
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
        S::Error: Source,
    {
        Ok(match value {
            None => RcWeakResolver {
                inner: RcResolver { pos: 0 },
            },
            Some(r) => RcWeakResolver {
                inner: ArchivedRc::<T, F>::serialize_from_ref(r, serializer)?,
            },
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
pub struct RcWeakResolver {
    inner: RcResolver,
}

#[cfg(feature = "bytecheck")]
mod verify {
    use core::{any::TypeId, error::Error, fmt};

    use bytecheck::{
        rancor::{Fallible, Source},
        CheckBytes, Verify,
    };
    use rancor::fail;

    use crate::{
        rc::{ArchivedRc, ArchivedRcWeak, Flavor},
        traits::{ArchivePointee, LayoutRaw},
        validation::{
            shared::ValidationState, ArchiveContext, ArchiveContextExt,
            SharedContext,
        },
    };

    #[derive(Debug)]
    struct CyclicSharedPointerError;

    impl fmt::Display for CyclicSharedPointerError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "encountered cyclic shared pointers while validating")
        }
    }

    impl Error for CyclicSharedPointerError {}

    unsafe impl<T, F, C> Verify<C> for ArchivedRc<T, F>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized + 'static,
        T::ArchivedMetadata: CheckBytes<C>,
        F: Flavor,
        C: Fallible + ArchiveContext + SharedContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr = self.ptr.as_ptr_wrapping();
            let type_id = TypeId::of::<ArchivedRc<T, F>>();

            let addr = ptr as *const u8 as usize;
            match context.start_shared(addr, type_id)? {
                ValidationState::Started => {
                    context.in_subtree(ptr, |context| unsafe {
                        T::check_bytes(ptr, context)
                    })?;
                    context.finish_shared(addr, type_id)?;
                }
                ValidationState::Pending => {
                    if !F::ALLOW_CYCLES {
                        fail!(CyclicSharedPointerError)
                    }
                }
                ValidationState::Finished => (),
            }

            Ok(())
        }
    }

    unsafe impl<T, F, C> Verify<C> for ArchivedRcWeak<T, F>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized + 'static,
        T::ArchivedMetadata: CheckBytes<C>,
        F: Flavor,
        C: Fallible + ArchiveContext + SharedContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            if self.ptr.is_invalid() {
                Ok(())
            } else {
                // SAFETY: `ArchivedRc` and `ArchivedRcWeak` are
                // `repr(transparent)` and so have the same layout as each
                // other.
                let rc = unsafe {
                    &*(self as *const Self).cast::<ArchivedRc<T, F>>()
                };
                rc.verify(context)
            }
        }
    }
}
