//! Wrapper type support and commonly used wrappers.

mod core;
#[cfg(feature = "std")]
mod std;

#[cfg(feature = "std")]
pub use self::std::*;

use crate::{Archive, Deserialize, Fallible, Serialize};
use ::core::{
    fmt,
    marker::PhantomData,
    mem::{transmute, MaybeUninit},
    ops::Deref,
};

/// A transparent wrapper for archived fields.
///
/// This is used by the `#[with(...)]` attribute to create transparent serialization wrappers.
#[repr(transparent)]
pub struct With<F: ?Sized, W> {
    _phantom: PhantomData<W>,
    field: F,
}

impl<F: ?Sized, W> With<F, W> {
    /// Casts a `With` reference from a reference to the underlying field.
    ///
    /// This is always safe to do because `With` is a transparent wrapper.
    #[inline]
    pub fn cast(field: &F) -> &'_ With<F, W> {
        // Safety: transmuting from an unsized type reference to a reference to a transparent
        // wrapper is safe because they both have the same data address and metadata
        #[allow(clippy::transmute_ptr_to_ptr)]
        unsafe {
            transmute(field)
        }
    }
}

impl<F, W> With<F, W> {
    /// Unwraps a `With` into the underlying field.
    #[inline]
    pub fn into_inner(self) -> F {
        self.field
    }
}

impl<F: ?Sized, W> AsRef<F> for With<F, W> {
    fn as_ref(&self) -> &F {
        &self.field
    }
}

/// A variant of `Archive` that works with `With` wrappers.
pub trait ArchiveWith<F: ?Sized> {
    /// The archived type of a `With<F, Self>`.
    type Archived;
    /// The resolver of a `With<F, Self>`.
    type Resolver;

    /// Resolves the archived type using a reference to the field type `F`.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `field`
    unsafe fn resolve_with(
        field: &F,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    );
}

impl<F: ?Sized, W: ArchiveWith<F>> Archive for With<F, W> {
    type Archived = W::Archived;
    type Resolver = W::Resolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    ) {
        let as_with = &mut *out.as_mut_ptr().cast();
        W::resolve_with(&self.field, pos, resolver, as_with);
    }
}

/// A variant of `Serialize` that works with `With` wrappers.
pub trait SerializeWith<F: ?Sized, S: Fallible + ?Sized>: ArchiveWith<F> {
    /// Serializes the field type `F` using the given serializer.
    fn serialize_with(field: &F, serializer: &mut S) -> Result<Self::Resolver, S::Error>;
}

impl<F: ?Sized, W: SerializeWith<F, S>, S: Fallible + ?Sized> Serialize<S> for With<F, W> {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        W::serialize_with(&self.field, serializer)
    }
}

/// A variant of `Deserialize` that works with `With` wrappers.
pub trait DeserializeWith<F: ?Sized, T, D: Fallible + ?Sized> {
    /// Deserializes the field type `F` using the given deserializer.
    fn deserialize_with(field: &F, deserializer: &mut D) -> Result<T, D::Error>;
}

impl<F, W, T, D> Deserialize<With<T, W>, D> for F
where
    F: ?Sized,
    W: DeserializeWith<F, T, D>,
    D: Fallible + ?Sized,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<With<T, W>, D::Error> {
        Ok(With {
            _phantom: PhantomData,
            field: W::deserialize_with(self, deserializer)?,
        })
    }
}

/// A wrapper to make a type immutable.
#[repr(transparent)]
pub struct Immutable<T: ?Sized>(T);

impl<T: ?Sized> Immutable<T> {
    /// Gets the underlying immutable value.
    #[inline]
    pub fn value(&self) -> &T {
        &self.0
    }
}

impl<T> Immutable<T> {
    /// Casts a `MaybeUninit<Immutable<T>>` to a `MaybeUninit<T>`.
    ///
    /// This is always safe because `Immutable` is a transparent wrapper.
    #[inline]
    pub fn map_inner(out: &mut MaybeUninit<Self>) -> &mut MaybeUninit<T> {
        unsafe { &mut *out.as_mut_ptr().cast() }
    }
}

impl<T: ?Sized> Deref for Immutable<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// A wrapper that serializes a reference inline.
pub struct Inline;

/// A wrapper that serializes a reference as if it were boxed.
pub struct Boxed;

/// A wrapper that attempts to convert a path to and from UTF-8.
pub struct AsString;

/// Errors that can occur when serializing a [`AsString`] wrapper.
#[derive(Debug)]
pub enum AsStringError {
    /// The `OsString` or `PathBuf` was not valid UTF-8.
    InvalidUTF8,
}

impl fmt::Display for AsStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UTF-8")
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for AsStringError {}

/// A wrapper that locks a lock and serializes the value immutably.
///
/// This wrapper can panic under very specific circumstances when:
///
/// 1. `serialize_with` is called and succeeds in locking the value to serialize it.
/// 2. Another thread locks the value and panics, poisoning the lock
/// 3. `resolve_with` is called and gets a poisoned value.
///
/// Unfortunately, it's not possible to work around this issue. If your code absolutely must not
/// panic under any circumstances, it's recommended that you lock your values and then serialize
/// them while locked.
pub struct Lock;

/// Errors that can occur while serializing a [`Lock`] wrapper
#[derive(Debug)]
pub enum LockError {
    /// The mutex was poisoned
    Poisoned,
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lock poisoned")
    }
}

#[cfg(feature = "std")]
impl ::std::error::Error for LockError {}
