//! A niched archived `Option<Box<T>>` that uses less space.

use core::{
    cmp, fmt, hash, hint::unreachable_unchecked, mem::ManuallyDrop, ops::Deref,
    pin::Pin,
};

use munge::munge;
use rancor::Fallible;

use crate::{
    boxed::{ArchivedBox, BoxResolver},
    ser::Writer,
    ArchivePointee, ArchiveUnsized, Place, Portable, RelPtr, SerializeUnsized,
};

/// A niched archived `Option<Box<T>>`.
///
/// It uses less space by storing the `None` variant as a null pointer.
#[derive(Portable)]
#[archive(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[repr(transparent)]
pub struct ArchivedOptionBox<T: ArchivePointee + ?Sized> {
    repr: Repr<T>,
}

#[derive(Portable)]
#[archive(crate)]
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
        validation::ArchiveContext,
        LayoutRaw,
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
            RelPtr::check_bytes(value.cast::<RelPtr<T>>(), context)?;

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
        #[inline]
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
    #[inline]
    pub fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }

    /// Returns `true` if the option box is a `Some` value.
    #[inline]
    pub fn is_some(&self) -> bool {
        self.as_ref().is_some()
    }

    /// Converts to an `Option<&ArchivedBox<T>>`.
    #[inline]
    pub fn as_ref(&self) -> Option<&ArchivedBox<T>> {
        if self.repr.is_invalid() {
            None
        } else {
            unsafe { Some(&self.repr.boxed) }
        }
    }

    /// Converts to an `Option<&mut ArchivedBox<T>>`.
    #[inline]
    pub fn as_mut(&mut self) -> Option<&mut ArchivedBox<T>> {
        if self.repr.is_invalid() {
            None
        } else {
            unsafe { Some(&mut self.repr.boxed) }
        }
    }

    /// Converts from `Pin<&ArchivedOptionBox<T>>` to
    /// `Option<Pin<&ArchivedBox<T>>>`.
    #[inline]
    pub fn as_pin_ref(self: Pin<&Self>) -> Option<Pin<&ArchivedBox<T>>> {
        unsafe { Pin::get_ref(self).as_ref().map(|x| Pin::new_unchecked(x)) }
    }

    /// Converts from `Pin<&mut ArchivedOption<T>>` to `Option<Pin<&mut
    /// ArchivedBox<T>>>`.
    #[inline]
    pub fn as_pin_mut(
        self: Pin<&mut Self>,
    ) -> Option<Pin<&mut ArchivedBox<T>>> {
        unsafe {
            Pin::get_unchecked_mut(self)
                .as_mut()
                .map(|x| Pin::new_unchecked(x))
        }
    }

    /// Returns an iterator over the possibly contained value.
    #[inline]
    pub fn iter(&self) -> Iter<'_, ArchivedBox<T>> {
        Iter {
            inner: self.as_ref(),
        }
    }

    /// Returns a mutable iterator over the possibly contained value.
    #[inline]
    pub fn iter_mut(&mut self) -> IterMut<'_, ArchivedBox<T>> {
        IterMut {
            inner: self.as_mut(),
        }
    }

    /// Converts from `&ArchivedOptionBox<T>` to `Option<&T>`.
    ///
    /// Leaves the original `ArchivedOptionBox` in-place, creating a new one
    /// with a reference to the original one.
    #[inline]
    pub fn as_deref(&self) -> Option<&T> {
        self.as_ref().map(|x| (*x).deref())
    }
}

impl<T: ArchivePointee + ?Sized> ArchivedOptionBox<T>
where
    T::ArchivedMetadata: Default,
{
    /// Resolves an `ArchivedOptionBox<T::Archived>` from an `Option<&T>`.
    #[inline]
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
    #[inline]
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
    #[inline]
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
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

impl<T: ArchivePointee + Ord + ?Sized> Ord for ArchivedOptionBox<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(&other.as_ref())
    }
}

impl<T: ArchivePointee + PartialEq + ?Sized> PartialEq
    for ArchivedOptionBox<T>
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq(&other.as_ref())
    }
}

impl<T: ArchivePointee + PartialOrd + ?Sized> PartialOrd
    for ArchivedOptionBox<T>
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ref().partial_cmp(&other.as_ref())
    }
}

/// An iterator over a reference to the `Some` variant of an
/// `ArchivedOptionBox`.
///
/// This iterator yields one value if the `ArchivedOptionBox` is a `Some`,
/// otherwise none.
///
/// This `struct` is created by the [`ArchivedOptionBox::iter`] function.
pub type Iter<'a, T> = crate::option::Iter<'a, T>;

/// An iterator over a mutable reference to the `Some` variant of an
/// `ArchivedOptionBox`.
///
/// This iterator yields one value if the `ArchivedOptionBox` is a `Some`,
/// otherwise none.
///
/// This `struct` is created by the [`ArchivedOptionBox::iter_mut`] function.
pub type IterMut<'a, T> = crate::option::IterMut<'a, T>;

/// The resolver for [`ArchivedOptionBox`].
pub enum OptionBoxResolver {
    /// The `ArchivedOptionBox` was `None`
    None,
    /// The resolver for the `ArchivedBox`
    Some(BoxResolver),
}

#[cfg(all(test, feature = "bytecheck"))]
mod tests {
    use crate::{rancor::Failure, Archived};

    #[test]
    fn test_option_box() {
        #[derive(Debug, crate::Archive, crate::Serialize)]
        #[archive(check_bytes, crate)]
        struct Test {
            #[with(crate::with::Niche)]
            value: Option<Box<u128>>,
        }

        for value in [Some(128.into()), None] {
            let test = Test { value };
            let bytes = crate::to_bytes::<Failure>(&test).unwrap();
            // ptr + value?
            assert_eq!(bytes.len(), 4 + test.value.is_some() as usize * 16);

            let ar = match crate::access::<Archived<Test>, Failure>(&bytes) {
                Ok(archived) => archived,
                Err(e) => panic!("{} {:?}", e, test),
            };

            match test.value {
                Some(value) => assert_eq!(
                    ar.value.as_ref().unwrap().as_ref(),
                    value.as_ref()
                ),
                None => assert!(ar.value.is_none()),
            }
        }
    }
}
