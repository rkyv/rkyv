//! A niched archived `Option<Box<T>>` that uses less space.

use core::{
    cmp, fmt, hash, hint::unreachable_unchecked, mem::ManuallyDrop, ops::Deref,
};

use munge::munge;
use rancor::Fallible;

use crate::{
    boxed::{ArchivedBox, BoxResolver},
    seal::Seal,
    ser::Writer,
    traits::ArchivePointee,
    ArchiveUnsized, Place, Portable, RelPtr, SerializeUnsized,
};

/// A niched archived `Option<Box<T>>`.
///
/// It uses less space by storing the `None` variant as a null pointer.
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedOptionBox<T: ArchivePointee + ?Sized> {
    repr: Repr<T>,
}

#[derive(Portable)]
#[rkyv(crate)]
#[repr(C)]
union Repr<T: ArchivePointee + ?Sized> {
    boxed: ManuallyDrop<ArchivedBox<T>>,
    ptr: ManuallyDrop<RelPtr<T>>,
}

impl<T: ArchivePointee + ?Sized> Repr<T> {
    fn is_invalid(&self) -> bool {
        unsafe { self.ptr.is_invalid() }
    }
}

#[cfg(feature = "bytecheck")]
const _: () = {
    use crate::{
        bytecheck::{CheckBytes, Verify},
        rancor::Source,
        traits::LayoutRaw,
        validation::ArchiveContext,
    };

    unsafe impl<T, C> CheckBytes<C> for Repr<T>
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

    unsafe impl<T, C> Verify<C> for Repr<T>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized,
        T::ArchivedMetadata: CheckBytes<C>,
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Source,
    {
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let is_invalid = unsafe { self.ptr.is_invalid() };
            if is_invalid {
                // This is a `None` and doesn't need to be checked further
                Ok(())
            } else {
                unsafe { self.boxed.verify(context) }
            }
        }
    }
};

impl<T: ArchivePointee + ?Sized> ArchivedOptionBox<T> {
    /// Returns `true` if the option box is a `None` value.
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }

    /// Returns `true` if the option box is a `Some` value.
    pub fn is_some(&self) -> bool {
        self.as_ref().is_some()
    }

    /// Converts to an `Option<&ArchivedBox<T>>`.
    pub fn as_ref(&self) -> Option<&ArchivedBox<T>> {
        if self.repr.is_invalid() {
            None
        } else {
            unsafe { Some(&self.repr.boxed) }
        }
    }

    /// Converts to an `Option<&mut ArchivedBox<T>>`.
    pub fn as_mut(&mut self) -> Option<&mut ArchivedBox<T>> {
        if self.repr.is_invalid() {
            None
        } else {
            unsafe { Some(&mut self.repr.boxed) }
        }
    }

    /// Converts from `Seal<'_, ArchivedOption<T>>` to `Option<Seal<'_,
    /// ArchivedBox<T>>>`.
    pub fn as_seal(this: Seal<'_, Self>) -> Option<Seal<'_, ArchivedBox<T>>> {
        let this = unsafe { Seal::unseal_unchecked(this) };
        this.as_mut().map(Seal::new)
    }

    /// Returns an iterator over the possibly-contained value.
    pub fn iter(&self) -> Iter<&'_ ArchivedBox<T>> {
        Iter::new(self.as_ref())
    }

    /// Returns an iterator over the mutable possibly-contained value.
    pub fn iter_mut(&mut self) -> Iter<&'_ mut ArchivedBox<T>> {
        Iter::new(self.as_mut())
    }

    /// Returns an iterator over the sealed possibly-contained value.
    pub fn iter_seal(this: Seal<'_, Self>) -> Iter<Seal<'_, ArchivedBox<T>>> {
        Iter::new(Self::as_seal(this))
    }

    /// Converts from `&ArchivedOptionBox<T>` to `Option<&T>`.
    ///
    /// Leaves the original `ArchivedOptionBox` in-place, creating a new one
    /// with a reference to the original one.
    pub fn as_deref(&self) -> Option<&T> {
        self.as_ref().map(|x| (*x).deref())
    }
}

impl<T: ArchivePointee + ?Sized> ArchivedOptionBox<T> {
    /// Resolves an `ArchivedOptionBox<T::Archived>` from an `Option<&T>`.
    pub fn resolve_from_option<U: ArchiveUnsized<Archived = T> + ?Sized>(
        field: Option<&U>,
        resolver: OptionBoxResolver,
        out: Place<Self>,
    ) {
        munge!(let Self { repr } = out);
        if let Some(value) = field {
            let resolver =
                if let OptionBoxResolver::Some(metadata_resolver) = resolver {
                    metadata_resolver
                } else {
                    unsafe {
                        unreachable_unchecked();
                    }
                };

            let out = unsafe { repr.cast_unchecked::<ArchivedBox<T>>() };
            ArchivedBox::resolve_from_ref(value, resolver, out)
        } else {
            let out = unsafe { repr.cast_unchecked::<RelPtr<T>>() };
            RelPtr::emplace_invalid(out);
        }
    }

    /// Serializes an `ArchivedOptionBox<T::Archived>` from an `Option<&T>`.
    pub fn serialize_from_option<U, S>(
        field: Option<&U>,
        serializer: &mut S,
    ) -> Result<OptionBoxResolver, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: Fallible + Writer + ?Sized,
    {
        if let Some(value) = field {
            Ok(OptionBoxResolver::Some(ArchivedBox::serialize_from_ref(
                value, serializer,
            )?))
        } else {
            Ok(OptionBoxResolver::None)
        }
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Debug for ArchivedOptionBox<T>
where
    T::ArchivedMetadata: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.as_ref() {
            Some(inner) => inner.fmt(f),
            None => f.debug_tuple("None").finish(),
        }
    }
}

impl<T: ArchivePointee + Eq + ?Sized> Eq for ArchivedOptionBox<T> {}

impl<T: ArchivePointee + hash::Hash + ?Sized> hash::Hash
    for ArchivedOptionBox<T>
{
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T: ArchivePointee + Ord + ?Sized> Ord for ArchivedOptionBox<T> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T: ArchivePointee + PartialEq + ?Sized> PartialEq
    for ArchivedOptionBox<T>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T: ArchivePointee + PartialOrd + ?Sized> PartialOrd
    for ArchivedOptionBox<T>
{
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ref().partial_cmp(&other.as_ref())
    }
}

/// An iterator over a reference to the `Some` variant of an
/// `ArchivedOptionBox`.
///
/// This iterator yields one value if the `ArchivedOptionBox` is a `Some`,
/// otherwise none.
pub type Iter<P> = crate::option::Iter<P>;

/// The resolver for [`ArchivedOptionBox`].
pub enum OptionBoxResolver {
    /// The `ArchivedOptionBox` was `None`
    None,
    /// The resolver for the `ArchivedBox`
    Some(BoxResolver),
}
