#[cfg(feature = "std")]
mod std_impl;

#[cfg(feature = "std")]
pub use std_impl::*;

use crate::{Archive, Deserialize, Fallible, Serialize};
use core::{marker::PhantomData, mem::MaybeUninit, ops::Deref};

/// A transparent wrapper for archived fields.
///
/// This is used by the `#[with(...)]` attribute to create transparent serialization wrappers.
#[repr(transparent)]
pub struct With<F, W> {
    _phantom: PhantomData<W>,
    field: F,
}

impl<F, W> With<F, W> {
    /// Casts a `With` reference from a reference to the underlying field.
    ///
    /// This is always safe to do because `With` is a transparent wrapper.
    #[inline]
    pub fn cast<'a>(field: &'a F) -> &'a With<F, W> {
        unsafe { &*(field as *const F).cast() }
    }

    /// Unwraps a `With` into the underlying field.
    #[inline]
    pub fn into_inner(self) -> F {
        self.field
    }
}

/// A variant of `Archive` that works with `With` wrappers.
pub trait ArchiveWith<F> {
    /// The archived type of a `With<F, Self>`.
    type Archived;
    /// The resolver of a `With<F, Self>`.
    type Resolver;

    /// Resolves the archived type using a reference to the field type `F`.
    fn resolve_with(
        field: &F,
        pos: usize,
        resolver: Self::Resolver,
        out: &mut MaybeUninit<Self::Archived>,
    );
}

impl<F, W: ArchiveWith<F>> Archive for With<F, W> {
    type Archived = W::Archived;
    type Resolver = W::Resolver;

    #[inline]
    fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        let as_with = unsafe { &mut *out.as_mut_ptr().cast() };
        W::resolve_with(&self.field, pos, resolver, as_with);
    }
}

/// A variant of `Serialize` that works with `With` wrappers.
pub trait SerializeWith<F, S: Fallible + ?Sized>: ArchiveWith<F> {
    /// Serializes the field type `F` using the given serializer.
    fn serialize_with(field: &F, serializer: &mut S) -> Result<Self::Resolver, S::Error>;
}

impl<F, W: SerializeWith<F, S>, S: Fallible + ?Sized> Serialize<S> for With<F, W> {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        W::serialize_with(&self.field, serializer)
    }
}

/// A variant of `Deserialize` that works with `With` wrappers.
pub trait DeserializeWith<F, T, D: Fallible + ?Sized> {
    /// Deserializes the field type `F` using the given deserializer.
    fn deserialize_with(field: &F, deserializer: &mut D) -> Result<T, D::Error>;
}

impl<F, W: DeserializeWith<F, T, D>, T, D: Fallible + ?Sized> Deserialize<With<T, W>, D> for F {
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
    pub fn as_inner(out: &mut MaybeUninit<Self>) -> &mut MaybeUninit<T> {
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
