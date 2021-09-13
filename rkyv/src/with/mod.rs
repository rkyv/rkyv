//! Wrapper type support and commonly used wrappers.
//!
//! Wrappers can be applied with the `#[with(...)]` attribute in the
//! [`Archive`](macro@crate::Archive) macro. See [`With`] for examples.

#[cfg(feature = "alloc")]
mod alloc;
mod atomic;
mod core;
#[cfg(feature = "std")]
mod std;

#[cfg(feature = "alloc")]
pub use self::alloc::*;
#[cfg(feature = "std")]
pub use self::std::*;

use crate::{Archive, Deserialize, Fallible, Serialize};
use ::core::{fmt, marker::PhantomData, mem::transmute, ops::Deref};

/// A transparent wrapper for archived fields.
///
/// This is used by the `#[with(...)]` attribute in the [`Archive`](macro@crate::Archive) macro to
/// create transparent serialization wrappers. Those wrappers leverage [`ArchiveWith`] to change
/// how the type is archived, serialized, and deserialized.
///
/// When a field is serialized, a reference to the field (i.e. `&T`) can be cast to a reference to a
/// `With` wrapper and serialized instead (i.e. `&With<T, Wrapper>`). This is safe to do because
/// `With` is a transparent wrapper and is shaped exactly the same as the underlying field.
///
/// # Example
///
/// ```
/// use rkyv::{Archive, with::Inline};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     // This will archive as if it were With<&'a i32, Inline>. That will delegate the archival
///     // to the ArchiveWith implementation of Inline for &T.
///     #[with(Inline)]
///     a: &'a i32,
/// }
/// ```
#[repr(transparent)]
#[derive(Debug)]
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

/// A variant of [`Archive`] that works with [`With`] wrappers.
///
/// Creating a wrapper allows users to customize how fields are archived easily without changing the
/// unarchived type.
///
/// This trait allows wrapper types to transparently change the archive behaviors for struct fields.
/// When a field is serialized, its reference may be converted to a [`With`] reference, and that
/// reference may be serialized instead. `With` references look for implementations of `ArchiveWith`
/// to determine how a wrapped field should be treated.
///
/// # Example
///
/// ```
/// use rkyv::{
///     archived_root,
///     ser::{
///         serializers::AllocSerializer,
///         Serializer,
///     },
///     with::{
///         ArchiveWith,
///         DeserializeWith,
///         SerializeWith,
///     },
///     Archive,
///     Archived,
///     Deserialize,
///     Fallible,
///     Infallible,
///     Resolver,
///     Serialize,
/// };
///
/// struct Incremented;
///
/// impl ArchiveWith<i32> for Incremented {
///     type Archived = Archived<i32>;
///     type Resolver = Resolver<i32>;
///
///     unsafe fn resolve_with(field: &i32, pos: usize, _: (), out: *mut Self::Archived) {
///         let incremented = field + 1;
///         incremented.resolve(pos, (), out);
///     }
/// }
///
/// impl<S: Fallible + ?Sized> SerializeWith<i32, S> for Incremented
/// where
///     i32: Serialize<S>,
/// {
///     fn serialize_with(field: &i32, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
///         let incremented = field + 1;
///         incremented.serialize(serializer)
///     }
/// }
///
/// impl<D: Fallible + ?Sized> DeserializeWith<Archived<i32>, i32, D> for Incremented
/// where
///     Archived<i32>: Deserialize<i32, D>,
/// {
///     fn deserialize_with(field: &Archived<i32>, deserializer: &mut D) -> Result<i32, D::Error> {
///         Ok(field.deserialize(deserializer)? - 1)
///     }
/// }
///
/// #[derive(Archive, Deserialize, Serialize)]
/// struct Example {
///     #[with(Incremented)]
///     a: i32,
///     // Another i32 field, but not incremented this time
///     b: i32,
/// }
///
/// let value = Example {
///     a: 4,
///     b: 9,
/// };
///
/// let mut serializer = AllocSerializer::<4096>::default();
/// serializer.serialize_value(&value).unwrap();
/// let buf = serializer.into_serializer().into_inner();
///
/// let archived = unsafe { archived_root::<Example>(buf.as_ref()) };
/// // The wrapped field has been incremented
/// assert_eq!(archived.a, 5);
/// // ... and the unwrapped field has not
/// assert_eq!(archived.b, 9);
///
/// let deserialized: Example = archived.deserialize(&mut Infallible).unwrap();
/// // The wrapped field is back to normal
/// assert_eq!(deserialized.a, 4);
/// // ... and the unwrapped field is unchanged
/// assert_eq!(deserialized.b, 9);
/// ```
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
        out: *mut Self::Archived,
    );
}

impl<F: ?Sized, W: ArchiveWith<F>> Archive for With<F, W> {
    type Archived = W::Archived;
    type Resolver = W::Resolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        W::resolve_with(&self.field, pos, resolver, out.cast());
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
#[derive(Debug)]
pub struct Immutable<T: ?Sized>(T);

impl<T: ?Sized> Immutable<T> {
    /// Gets the underlying immutable value.
    #[inline]
    pub fn value(&self) -> &T {
        &self.0
    }
}

impl<T: ?Sized> Deref for Immutable<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "validation")]
const _: () = {
    use bytecheck::CheckBytes;

    impl<T: CheckBytes<C> + ?Sized, C: ?Sized> CheckBytes<C> for Immutable<T> {
        type Error = T::Error;

        unsafe fn check_bytes<'a>(
            value: *const Self,
            context: &mut C,
        ) -> Result<&'a Self, Self::Error> {
            CheckBytes::check_bytes(::core::ptr::addr_of!((*value).0), context)?;
            Ok(&*value)
        }
    }
};

/// A wrapper that archives an atomic with an underlying atomic.
///
/// By default, atomics are archived with an underlying integer.
///
/// # Safety
///
/// This wrapper is only safe to use when the backing memory for wrapped types is mutable.
///
/// # Example
///
/// ```
/// # #[cfg(has_atomics)]
/// use core::sync::atomic::AtomicU32;
/// use rkyv::{Archive, with::Atomic};
///
/// # #[cfg(has_atomics)]
/// #[derive(Archive)]
/// struct Example {
///     #[with(Atomic)]
///     a: AtomicU32,
/// }
/// ```
#[derive(Debug)]
pub struct Atomic;

/// A wrapper that serializes a reference inline.
///
/// References serialized with `Inline` cannot be deserialized because the struct cannot own the
/// deserialized value.
///
/// # Example
///
/// ```
/// use rkyv::{Archive, with::Inline};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(Inline)]
///     a: &'a i32,
/// }
/// ```
#[derive(Debug)]
pub struct Inline;

/// A wrapper that serializes a reference as if it were boxed.
///
/// Unlike [`Inline`], unsized references can be serialized with `Boxed`.
///
/// References serialized with `Boxed` cannot be deserialized because the struct cannot own the
/// deserialized value.
///
/// # Example
///
/// ```
/// use rkyv::{Archive, with::Boxed};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(Boxed)]
///     a: &'a str,
/// }
/// ```
#[derive(Debug)]
pub struct Boxed;

/// A wrapper that attempts to convert a type to and from UTF-8.
///
/// Types like `OsString` and `PathBuf` aren't guaranteed to be encoded as UTF-8, but they usually
/// are anyway. Using this wrapper will archive them as if they were regular `String`s.
///
/// # Example
///
/// ```
/// use std::{ffi::OsString, path::PathBuf};
/// use rkyv::{Archive, with::AsString};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsString)]
///     os_string: OsString,
///     #[with(AsString)]
///     path: PathBuf,
/// }
/// ```
#[derive(Debug)]
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
///
/// # Example
///
/// ```
/// use std::sync::Mutex;
/// use rkyv::{Archive, with::Lock};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(Lock)]
///     a: Mutex<i32>,
/// }
/// ```
#[derive(Debug)]
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

/// A wrapper that serializes a `Cow` as if it were owned.
///
/// # Example
///
/// ```
/// use std::borrow::Cow;
/// use rkyv::{Archive, with::AsOwned};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(AsOwned)]
///     a: Cow<'a, str>,
/// }
/// ```
#[derive(Debug)]
pub struct AsOwned;

/// A wrapper that serializes associative containers as a `Vec` of key-value pairs.
///
/// This provides faster serialization for containers like `HashMap` and `BTreeMap` by serializing
/// the key-value pairs directly instead of building a data structure in the buffer.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use rkyv::{Archive, with::AsVec};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsVec)]
///     values: HashMap<String, u32>,
/// }
/// ```
#[derive(Debug)]
pub struct AsVec;

/// A wrapper that niches some type combinations.
///
/// A common type combination is `Option<Box<T>>`. By using a null pointer, the archived version can
/// save some space on-disk.
#[derive(Debug)]
pub struct Niche;
