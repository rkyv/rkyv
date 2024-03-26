//! An archived version of `Box`.

use core::{borrow::Borrow, cmp, fmt, hash, ops::Deref, pin::Pin};

use rancor::Fallible;

use crate::{
    ser::{Writer, WriterExt as _},
    ArchivePointee, ArchiveUnsized, Portable, RelPtr, Serialize,
    SerializeUnsized,
};

/// An archived [`Box`].
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type.
#[derive(Portable)]
#[archive(crate)]
#[cfg_attr(
    feature = "bytecheck",
    derive(bytecheck::CheckBytes),
    check_bytes(verify)
)]
#[repr(transparent)]
pub struct ArchivedBox<T: ArchivePointee + ?Sized> {
    ptr: RelPtr<T>,
}

impl<T: ArchivePointee + ?Sized> ArchivedBox<T> {
    /// Returns a reference to the value of this archived box.
    #[inline]
    pub fn get(&self) -> &T {
        unsafe { &*self.ptr.as_ptr() }
    }

    /// Returns a pinned mutable reference to the value of this archived box
    #[inline]
    pub fn get_pin_mut(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|s| &mut *s.ptr.as_ptr()) }
    }

    /// Resolves an archived box from the given value and parameters.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        pos: usize,
        resolver: BoxResolver,
        out: *mut Self,
    ) {
        Self::resolve_from_raw_parts(
            pos,
            resolver,
            value.archived_metadata(),
            out,
        )
    }

    /// Serializes an archived box from the given value and serializer.
    #[inline]
    pub fn serialize_from_ref<U, S>(
        value: &U,
        serializer: &mut S,
    ) -> Result<BoxResolver, S::Error>
    where
        U: SerializeUnsized<S, Archived = T> + ?Sized,
        S: Fallible + ?Sized,
    {
        Ok(BoxResolver {
            pos: value.serialize_unsized(serializer)?,
        })
    }

    /// Resolves an archived box from a [`BoxResolver`] and the raw metadata
    /// directly.
    ///
    /// # Safety
    ///
    /// `out` must point to a `Self` that is valid for reads and writes.
    pub unsafe fn resolve_from_raw_parts(
        pos: usize,
        resolver: BoxResolver,
        metadata: T::ArchivedMetadata,
        out: *mut Self,
    ) {
        let (fp, fo) = out_field!(out.ptr);
        RelPtr::emplace_unsized(pos + fp, resolver.pos, metadata, fo);
    }

    #[doc(hidden)]
    #[inline]
    pub fn is_null(&self) -> bool {
        self.ptr.is_null()
    }
}

impl<T> ArchivedBox<[T]> {
    /// Serializes an archived `Box` from a given slice by directly copying
    /// bytes.
    ///
    /// # Safety
    ///
    /// The type being serialized must be copy-safe. Copy-safe types must be
    /// trivially copyable (have the same archived and unarchived
    /// representations) and contain no padding bytes. In situations where
    /// copying uninitialized bytes the output is acceptable, this function may
    /// be used with types that contain padding bytes.
    #[inline]
    pub unsafe fn serialize_copy_from_slice<U, S>(
        slice: &[U],
        serializer: &mut S,
    ) -> Result<BoxResolver, S::Error>
    where
        U: Serialize<S, Archived = T>,
        S: Fallible + Writer + ?Sized,
    {
        use core::{mem::size_of, slice::from_raw_parts};

        let pos = serializer.align_for::<T>()?;

        let bytes = from_raw_parts(
            slice.as_ptr().cast::<u8>(),
            size_of::<T>() * slice.len(),
        );
        serializer.write(bytes)?;

        Ok(BoxResolver { pos })
    }
}

impl<T: ArchivePointee + ?Sized> ArchivedBox<T>
where
    T::ArchivedMetadata: Default,
{
    #[doc(hidden)]
    #[inline]
    pub unsafe fn emplace_null(pos: usize, out: *mut Self) {
        let (fp, fo) = out_field!(out.ptr);
        RelPtr::emplace_null(pos + fp, fo);
    }
}

impl<T: ArchivePointee + ?Sized> AsRef<T> for ArchivedBox<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> Borrow<T> for ArchivedBox<T> {
    #[inline]
    fn borrow(&self) -> &T {
        self.get()
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Debug for ArchivedBox<T>
where
    T::ArchivedMetadata: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArchivedBox").field(&self.ptr).finish()
    }
}

impl<T: ArchivePointee + ?Sized> Deref for ArchivedBox<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

impl<T: ArchivePointee + fmt::Display + ?Sized> fmt::Display
    for ArchivedBox<T>
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.get().fmt(f)
    }
}

impl<T: ArchivePointee + Eq + ?Sized> Eq for ArchivedBox<T> {}

impl<T: ArchivePointee + hash::Hash + ?Sized> hash::Hash for ArchivedBox<T> {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.get().hash(state);
    }
}

impl<T: ArchivePointee + Ord + ?Sized> Ord for ArchivedBox<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ArchivePointee + ?Sized>
    PartialEq<ArchivedBox<U>> for ArchivedBox<T>
{
    #[inline]
    fn eq(&self, other: &ArchivedBox<U>) -> bool {
        self.get().eq(other.get())
    }
}

impl<T: ArchivePointee + PartialOrd + ?Sized> PartialOrd for ArchivedBox<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.get().partial_cmp(other.get())
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Pointer for ArchivedBox<T> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.get() as *const T;
        fmt::Pointer::fmt(&ptr, f)
    }
}

/// The resolver for `Box`.
pub struct BoxResolver {
    pos: usize,
}

impl BoxResolver {
    /// Creates a new [`BoxResolver`] from the position of a serialized value.
    ///
    /// In most cases, you won't need to create a [`BoxResolver`] yourself and
    /// can instead obtain it through [`ArchivedBox::serialize_from_ref`] or
    /// [`ArchivedBox::serialize_copy_from_slice`].
    pub fn from_pos(pos: usize) -> Self {
        Self { pos }
    }
}

#[cfg(feature = "bytecheck")]
mod verify {
    use bytecheck::{
        rancor::{Error, Fallible},
        CheckBytes, Verify,
    };

    use crate::{
        boxed::ArchivedBox,
        validation::{ArchiveContext, ArchiveContextExt},
        ArchivePointee, LayoutRaw,
    };

    unsafe impl<T, C> Verify<C> for ArchivedBox<T>
    where
        T: ArchivePointee + CheckBytes<C> + LayoutRaw + ?Sized,
        T::ArchivedMetadata: CheckBytes<C>,
        C: Fallible + ArchiveContext + ?Sized,
        C::Error: Error,
    {
        #[inline]
        fn verify(&self, context: &mut C) -> Result<(), C::Error> {
            let ptr =
                unsafe { context.bounds_check_subtree_rel_ptr(&self.ptr)? };

            let range = unsafe { context.push_prefix_subtree(ptr)? };
            unsafe {
                T::check_bytes(ptr, context)?;
            }
            unsafe {
                context.pop_subtree_range(range)?;
            }

            Ok(())
        }
    }
}
