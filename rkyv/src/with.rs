//! Wrapper type support and commonly used wrappers.
//!
//! Wrappers can be applied with the `#[rkyv(with = ..)]` attribute in the
//! [`Archive`](macro@crate::Archive) macro.

// mod impls;

use core::{fmt, marker::PhantomData};

use rancor::Fallible;

#[doc(inline)]
pub use crate::niche::niching::DefaultNiche;
use crate::{Archive, Deserialize, Place, Portable, Serialize};

/// A variant of [`Archive`] that works with wrappers.
///
/// Creating a wrapper allows users to customize how fields are archived easily
/// without changing the unarchived type.
///
/// This trait allows wrapper types to transparently change the archive
/// behaviors for struct and enum fields. When a field is serialized, it may use
/// the implementations for the wrapper type and the given field instead of the
/// implementation for the type itself.
///
/// Only a single implementation of [`Archive`] may be written
/// for each type, but multiple implementations of ArchiveWith can be written
/// for the same type because it is parametric over the wrapper type. This is
/// used with the `#[rkyv(with = ..)]` macro attribute to provide a more
/// flexible interface for serialization.
///
/// # Example
///
/// ```
/// use rkyv::{
///     access_unchecked, deserialize,
///     rancor::{Error, Fallible, Infallible, ResultExt as _},
///     to_bytes,
///     with::{ArchiveWith, DeserializeWith, SerializeWith},
///     Archive, Archived, Deserialize, Place, Resolver, Serialize,
/// };
///
/// struct Incremented;
///
/// impl ArchiveWith<i32> for Incremented {
///     type Archived = Archived<i32>;
///     type Resolver = Resolver<i32>;
///
///     fn resolve_with(field: &i32, _: (), out: Place<Self::Archived>) {
///         let incremented = field + 1;
///         incremented.resolve((), out);
///     }
/// }
///
/// impl<S> SerializeWith<i32, S> for Incremented
/// where
///     S: Fallible + ?Sized,
///     i32: Serialize<S>,
/// {
///     fn serialize_with(
///         field: &i32,
///         serializer: &mut S,
///     ) -> Result<Self::Resolver, S::Error> {
///         let incremented = field + 1;
///         incremented.serialize(serializer)
///     }
/// }
///
/// impl<D> DeserializeWith<Archived<i32>, i32, D> for Incremented
/// where
///     D: Fallible + ?Sized,
///     Archived<i32>: Deserialize<i32, D>,
/// {
///     fn deserialize_with(
///         field: &Archived<i32>,
///         deserializer: &mut D,
///     ) -> Result<i32, D::Error> {
///         Ok(field.deserialize(deserializer)? - 1)
///     }
/// }
///
/// #[derive(Archive, Deserialize, Serialize)]
/// struct Example {
///     #[rkyv(with = Incremented)]
///     a: i32,
///     // Another i32 field, but not incremented this time
///     b: i32,
/// }
///
/// let value = Example { a: 4, b: 9 };
///
/// let buf = to_bytes::<Error>(&value).unwrap();
///
/// let archived =
///     unsafe { access_unchecked::<Archived<Example>>(buf.as_ref()) };
/// // The wrapped field has been incremented
/// assert_eq!(archived.a, 5);
/// // ... and the unwrapped field has not
/// assert_eq!(archived.b, 9);
///
/// let deserialized = deserialize::<Example, Infallible>(archived).always_ok();
/// // The wrapped field is back to normal
/// assert_eq!(deserialized.a, 4);
/// // ... and the unwrapped field is unchanged
/// assert_eq!(deserialized.b, 9);
/// ```
pub trait ArchiveWith<F: ?Sized> {
    /// The archived type of `Self` with `F`.
    type Archived: Portable;
    /// The resolver of a `Self` with `F`.
    type Resolver;

    /// Resolves the archived type using a reference to the field type `F`.
    fn resolve_with(
        field: &F,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    );
}

/// A variant of `Serialize` for "with" types.
///
/// See [ArchiveWith] for more details.
pub trait SerializeWith<F: ?Sized, S: Fallible + ?Sized>:
    ArchiveWith<F>
{
    /// Serializes the field type `F` using the given serializer.
    fn serialize_with(
        field: &F,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error>;
}

/// A variant of `Deserialize` for "with" types.
///
/// See [ArchiveWith] for more details.
pub trait DeserializeWith<F: ?Sized, T, D: Fallible + ?Sized> {
    /// Deserializes the field type `F` using the given deserializer.
    fn deserialize_with(field: &F, deserializer: &mut D)
        -> Result<T, D::Error>;
}

/// A transparent wrapper which applies a "with" type.
///
/// `With` wraps a reference to a type and applies the specified wrapper type
/// when serializing and deserializing.
#[repr(transparent)]
pub struct With<F: ?Sized, W> {
    _phantom: PhantomData<W>,
    field: F,
}

impl<F: ?Sized, W> With<F, W> {
    /// Casts a `With` reference from a reference to the underlying field.
    pub fn cast(field: &F) -> &Self {
        // SAFETY: `With` is `repr(transparent)` and so a reference to `F` can
        // always be transmuted into a reference to `With<F, W>`.
        unsafe { ::core::mem::transmute::<&F, &Self>(field) }
    }
}

impl<F: ?Sized, W: ArchiveWith<F>> Archive for With<F, W> {
    type Archived = <W as ArchiveWith<F>>::Archived;
    type Resolver = <W as ArchiveWith<F>>::Resolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        W::resolve_with(&self.field, resolver, out);
    }
}

impl<S, F, W> Serialize<S> for With<F, W>
where
    S: Fallible + ?Sized,
    F: ?Sized,
    W: SerializeWith<F, S>,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, <S as Fallible>::Error> {
        W::serialize_with(&self.field, serializer)
    }
}

impl<T, D, F, W> Deserialize<T, D> for With<F, W>
where
    D: Fallible + ?Sized,
    F: ?Sized,
    W: DeserializeWith<F, T, D>,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<T, <D as Fallible>::Error> {
        W::deserialize_with(&self.field, deserializer)
    }
}

/// A wrapper that applies another wrapper to the values contained in a type.
/// This can be applied to a vector to map each element, or an option to map any
/// contained value.
///
/// See [ArchiveWith] for more details.
///
/// # Example
///
/// ```
/// use rkyv::{
///     with::{InlineAsBox, Map},
///     Archive,
/// };
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     // This will apply `InlineAsBox` to the `&i32` contained in this option
///     #[rkyv(with = Map<InlineAsBox>)]
///     option: Option<&'a i32>,
///     // This will apply `InlineAsBox` to each `&i32` contained in this vector
///     #[rkyv(with = Map<InlineAsBox>)]
///     vec: Vec<&'a i32>,
/// }
/// ```
pub struct Map<T> {
    _phantom: PhantomData<T>,
}

/// A wrapper that applies key and value wrappers to the key-value pairs
/// contained in a type. This can be applied to a hash map or B-tree map to map
/// the key-value pairs.
///
/// # Example
/// ```
/// use std::collections::HashMap;
///
/// use rkyv::{
///     with::{Inline, InlineAsBox, MapKV},
///     Archive,
/// };
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     // This will apply `InlineAsBox` to the `&str` key, and `Inline` to the
///     // `&str` value.
///     #[rkyv(with = MapKV<InlineAsBox, Inline>)]
///     hash_map: HashMap<&'a str, &'a str>,
/// }
/// ```
pub struct MapKV<K, V> {
    _phantom: PhantomData<(K, V)>,
}

/// A type indicating relaxed atomic loads.
pub struct Relaxed;

/// A type indicating acquire atomic loads.
pub struct Acquire;

/// A type indicating sequentially-consistent atomic loads.
pub struct SeqCst;

/// A wrapper that archives an atomic by loading its value with a particular
/// ordering.
///
/// When serializing, the specified ordering will be used to load the value from
/// the source atomic. The underlying archived type is still a non-atomic value.
///
/// # Example
///
/// ```
/// # #[cfg(target_has_atomic = "32")]
/// use core::sync::atomic::AtomicU32;
///
/// use rkyv::{
///     with::{AtomicLoad, Relaxed},
///     Archive,
/// };
///
/// # #[cfg(target_has_atomic = "32")]
/// #[derive(Archive)]
/// struct Example {
///     #[rkyv(with = AtomicLoad<Relaxed>)]
///     a: AtomicU32,
/// }
/// ```
#[derive(Debug)]
pub struct AtomicLoad<SO> {
    _phantom: PhantomData<SO>,
}

/// A wrapper that serializes a reference inline.
///
/// References serialized with `Inline` cannot be deserialized because the
/// struct cannot own the deserialized value.
///
/// # Example
///
/// ```
/// use rkyv::{with::Inline, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[rkyv(with = Inline)]
///     a: &'a i32,
/// }
/// ```
#[derive(Debug)]
pub struct Inline;

/// A wrapper that serializes a field into a box.
///
/// This functions similarly to [`InlineAsBox`], but is for regular fields
/// instead of references.
///
/// # Example
///
/// ```
/// use rkyv::{with::AsBox, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[rkyv(with = AsBox)]
///     a: i32,
///     #[rkyv(with = AsBox)]
///     b: str,
/// }
/// ```
#[derive(Debug)]
pub struct AsBox;

/// A wrapper that serializes a reference as if it were boxed.
///
/// Unlike [`Inline`], unsized references can be serialized with `InlineAsBox`.
///
/// References serialized with `InlineAsBox` cannot be deserialized because the
/// struct cannot own the deserialized value.
///
/// # Example
///
/// ```
/// use rkyv::{with::InlineAsBox, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[rkyv(with = InlineAsBox)]
///     a: &'a i32,
///     #[rkyv(with = InlineAsBox)]
///     b: &'a str,
/// }
/// ```
#[derive(Debug)]
pub struct InlineAsBox;

/// A wrapper that attempts to convert a type to and from UTF-8.
///
/// Types like `OsString` and `PathBuf` aren't guaranteed to be encoded as
/// UTF-8, but they usually are anyway. Using this wrapper will archive them as
/// if they were regular `String`s.
///
/// It also allows `&str` to be archived like an owned `String`. However, note
/// that `&str` cannot be *deserialized* this way.
///
/// # Example
///
/// ```
/// use std::{ffi::OsString, path::PathBuf};
///
/// use rkyv::{with::AsString, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[rkyv(with = AsString)]
///     os_string: OsString,
///     #[rkyv(with = AsString)]
///     path: PathBuf,
///     #[rkyv(with = AsString)]
///     reference: &'a str,
/// }
/// ```
#[derive(Debug)]
pub struct AsString;

/// A wrapper that locks a lock and serializes the value immutably.
///
/// This wrapper can panic under very specific circumstances when:
///
/// 1. `serialize_with` is called and succeeds in locking the value to serialize
///    it.
/// 2. Another thread locks the value and panics, poisoning the lock
/// 3. `resolve_with` is called and gets a poisoned value.
///
/// Unfortunately, it's not possible to work around this issue internally. Users
/// must ensure this doesn't happen on their own through manual synchronization
/// or guaranteeing that panics do not occur while holding locks.
///
/// # Example
///
/// ```
/// use std::sync::Mutex;
///
/// use rkyv::{with::Lock, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[rkyv(with = Lock)]
///     a: Mutex<i32>,
/// }
/// ```
#[derive(Debug)]
pub struct Lock;

/// A wrapper that serializes a `Cow` as if it were owned.
///
/// # Example
///
/// ```
/// use std::borrow::Cow;
///
/// use rkyv::{with::AsOwned, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[rkyv(with = AsOwned)]
///     a: Cow<'a, str>,
/// }
/// ```
#[derive(Debug)]
pub struct AsOwned;

/// A wrapper that serializes associative containers as a `Vec` of key-value
/// pairs.
///
/// This provides faster serialization for containers like `HashMap` and
/// `BTreeMap` by serializing the key-value pairs directly instead of building a
/// data structure in the buffer.
///
/// It also allows `&[T]` to be archived like an owned `Vec`. However, note
/// that `&[T]` cannot be *deserialized* this way.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// use rkyv::{with::AsVec, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[rkyv(with = AsVec)]
///     values: HashMap<String, u32>,
///     #[rkyv(with = AsVec)]
///     slice: &'a [u32],
/// }
/// ```
#[derive(Debug)]
pub struct AsVec;

/// A wrapper that niches some type combinations.
///
/// A common type combination is `Option<Box<T>>`. By using a null pointer, the
/// archived version can save some space on-disk.
///
/// # Example
///
/// ```
/// use core::mem::size_of;
///
/// use rkyv::{with::Niche, Archive, Archived};
///
/// #[derive(Archive)]
/// struct BasicExample {
///     value: Option<Box<str>>,
/// }
///
/// #[derive(Archive)]
/// struct NichedExample {
///     #[rkyv(with = Niche)]
///     value: Option<Box<str>>,
/// }
///
/// assert!(
///     size_of::<Archived<BasicExample>>()
///         > size_of::<Archived<NichedExample>>()
/// );
/// ```
#[derive(Debug)]
pub struct Niche;

/// A wrapper that niches based on a generic [`Niching`].
///
/// A common type combination is `Option<Box<T>>`. By niching `None` into the
/// null pointer, the archived version can save some space on-disk.
///
/// # Example
///
/// ```
/// use core::mem::size_of;
///
/// use rkyv::{
///     niche::niching::{NaN, Null},
///     with::NicheInto,
///     Archive, Archived,
/// };
///
/// #[derive(Archive)]
/// struct BasicExample {
///     maybe_box: Option<Box<str>>,
///     maybe_non_nan: Option<f32>,
/// }
///
/// #[derive(Archive)]
/// struct NichedExample {
///     #[rkyv(with = NicheInto<Null>)]
///     maybe_box: Option<Box<str>>,
///     #[rkyv(with = NicheInto<NaN>)]
///     maybe_non_nan: Option<f32>,
/// }
///
/// assert!(
///     size_of::<Archived<BasicExample>>()
///         > size_of::<Archived<NichedExample>>()
/// );
/// ```
///
/// [`Niching`]: crate::niche::niching::Niching
pub struct NicheInto<N: ?Sized>(PhantomData<N>);

impl<N: ?Sized> Default for NicheInto<N> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<N: ?Sized> fmt::Debug for NicheInto<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NicheInto")
    }
}

/// A wrapper that first applies another wrapper `W` to the value inside an
/// `Option` and then niches the result based on the [`Niching`] `N`.
///
/// # Example
///
/// ```
/// use rkyv::{
///     with::{AsBox, MapNiche},
///     Archive, Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
/// struct BasicExample {
///     option: Option<HugeType>,
/// }
///
/// #[derive(Archive, Serialize)]
/// struct NichedExample {
///     #[rkyv(with = MapNiche<AsBox>)]
///     option: Option<HugeType>,
/// }
///
/// #[derive(Archive, Serialize)]
/// struct HugeType([u8; 1024]);
///
/// # fn main() -> Result<(), rkyv::rancor::Error> {
/// let basic_value = BasicExample { option: None };
/// let basic_bytes = rkyv::to_bytes(&basic_value)?;
/// assert_eq!(basic_bytes.len(), 1 + 1024);
///
/// let niched_value = NichedExample { option: None };
/// let niched_bytes = rkyv::to_bytes(&niched_value)?;
/// assert_eq!(niched_bytes.len(), 4); // size_of::<ArchivedBox<_>>()
/// # Ok(()) }
/// ```
///
/// [`Niching`]: crate::niche::niching::Niching
pub struct MapNiche<W: ?Sized, N: ?Sized = DefaultNiche> {
    _map: PhantomData<W>,
    _niching: PhantomData<N>,
}

impl<W: ?Sized, N: ?Sized> Default for MapNiche<W, N> {
    fn default() -> Self {
        Self {
            _map: PhantomData,
            _niching: PhantomData,
        }
    }
}

impl<W: ?Sized, N: ?Sized> fmt::Debug for MapNiche<W, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("MapNiche")
    }
}

/// A wrapper that converts a [`SystemTime`](std::time::SystemTime) to a
/// [`Duration`](std::time::Duration) since
/// [`UNIX_EPOCH`](std::time::UNIX_EPOCH).
///
/// If the serialized time occurs before the UNIX epoch, serialization will
/// panic during `resolve`. The resulting archived time will be an
/// [`ArchivedDuration`](crate::time::ArchivedDuration) relative to the UNIX
/// epoch.
///
/// # Example
///
/// ```
/// use rkyv::{Archive, with::AsUnixTime};
/// use std::time::SystemTime;
///
/// #[derive(Archive)]
/// struct Example {
///     #[rkyv(with = AsUnixTime)]
///     time: SystemTime,
/// }
#[derive(Debug)]
pub struct AsUnixTime;

/// A wrapper that allows serialize-unsafe types to be serialized.
///
/// Types like `Cell` and `UnsafeCell` may contain serializable types, but have
/// unsafe access semantics due to interior mutability. They may be safe to
/// serialize, but only under conditions that rkyv is unable to guarantee.
///
/// This wrapper enables serializing these types, and places the burden of
/// verifying that their access semantics are used safely on the user.
///
/// # Safety
///
/// Using this wrapper on types with interior mutability can create races
/// conditions or allow access to data in an invalid state if access semantics
/// are not followed properly. During serialization, the data must not be
/// modified.
///
/// # Example
///
/// ```
/// use core::cell::{Cell, UnsafeCell};
///
/// use rkyv::{with::Unsafe, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[rkyv(with = Unsafe)]
///     cell: Cell<String>,
///     #[rkyv(with = Unsafe)]
///     unsafe_cell: UnsafeCell<String>,
/// }
/// ```
#[derive(Debug)]
pub struct Unsafe;

/// A wrapper that skips serializing a field.
///
/// Skipped fields must implement `Default` to be deserialized.
///
/// # Example
///
/// ```
/// use rkyv::{with::Skip, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[rkyv(with = Skip)]
///     a: u32,
/// }
/// ```
#[derive(Debug)]
pub struct Skip;

/// A wrapper that clones the contents of `Arc` and `Rc` pointers.
#[derive(Debug)]
pub struct Unshare;

/// A no-op wrapper which uses the default impls for the type.
///
/// This is most useful for wrappers like [`MapKV`] when you only want to apply
/// a wrapper to either the key or the value.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// use rkyv::{
///     with::{Identity, Inline, MapKV},
///     Archive,
/// };
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[rkyv(with = MapKV<Identity, Inline>)]
///     a: HashMap<u32, &'a u32>,
/// }
/// ```
#[derive(Debug)]
pub struct Identity;
