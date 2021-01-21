//! # rkyv
//!
//! rkyv (*archive*) is a zero-copy deserialization framework for Rust.
//!
//! It's similar to other zero-copy deserialization frameworks such as
//! [Cap'n Proto](https://capnproto.org) and
//! [FlatBuffers](https://google.github.io/flatbuffers). However, while the
//! former have external schemas and heavily restricted data types, rkyv allows
//! all serialized types to be defined in code and can serialize a wide variety
//! of types that the others cannot. Additionally, rkyv is designed to have
//! little to no overhead, and in most cases will perform exactly the same as
//! native types.
//!
//! rkyv has a hashmap implementation that is built for zero-copy
//! deserialization, so you can serialize your hashmaps with abandon. The
//! implementation performs perfect hashing with the compress, hash and displace
//! algorithm to use as little memory as possible while still performing fast
//! lookups.
//!
//! rkyv also has support for contextual serialization, deserialization, and
//! validation. It can properly serialize and deserialize shared pointers like
//! `Rc` and `Arc`, and can be extended to support custom contextual types.
//!
//! One of the most impactful features made possible by rkyv is the ability to
//! serialize trait objects and use them *as trait objects* without
//! deserialization. See the `archive_dyn` crate for more details.
//!
//! ## Design
//!
//! Like [serde](https://serde.rs), rkyv uses Rust's powerful trait system to
//! serialize data without the need for reflection. Despite having a wide array
//! of features, you also only pay for what you use. If your data checks out,
//! the serialization process can be as simple as a `memcpy`! Like serde, this
//! allows rkyv to perform at speeds similar to handwritten serializers.
//!
//! Unlike serde, rkyv produces data that is guaranteed deserialization free. If
//! you wrote your data to disk, you can just `mmap` your file into memory, cast
//! a pointer, and your data is ready to use. This makes it ideal for
//! high-performance and IO-bound applications.
//!
//! Limited data mutation is supported through `Pin` APIs. Archived values can
//! be truly deserialized with [`Deserialize`] if full mutation capabilities are
//! needed.
//!
//! ## Tradeoffs
//!
//! rkyv is designed primarily for loading bulk game data as efficiently as
//! possible. While rkyv is a great format for final data, it lacks a full
//! schema system and isn't well equipped for data migration. Using a
//! serialization library like serde can help fill these gaps, and you can use
//! serde with the same types as rkyv conflict-free.
//!
//! ## Features
//!
//! - `const_generics`: Improves the trait implementations for arrays with
//!   support for all lengths
//! - `long_rel_ptrs`: Increases the size of relative pointers to 64 bits for
//!   large archive support
//! - `std`: Enables standard library support (enabled by default)
//! - `strict`: Guarantees that types will have the same representations across
//!   platforms and compilations. This is already the case in practice, but this
//!   feature provides a guarantee. It additionally provides C type
//!   compatibility.
//! - `validation`: Enables validation support through `bytecheck`
//!
//! ## Examples
//!
//! See [`Archive`] for examples of how to use rkyv.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "const_generics", allow(incomplete_features))]
#![cfg_attr(feature = "const_generics", feature(const_generics))]

pub mod core_impl;
pub mod de;
pub mod ser;
#[cfg(feature = "std")]
pub mod std_impl;
#[cfg(feature = "validation")]
pub mod validation;

use core::{marker::PhantomPinned, ops::{Deref, DerefMut}, pin::Pin};

pub use memoffset::offset_of;
pub use rkyv_derive::{Archive, Deserialize, Serialize};
#[cfg(feature = "validation")]
pub use validation::check_archive;

/// Writes a type to a [`Serializer`] so it can be used without deserializing.
///
/// Archiving is done depth-first, writing any data owned by a type before
/// writing the data for the type itself. The type must be able to create the
/// archived type from only its own data and its resolver.
///
/// ## Examples
///
/// Most of the time, `#[derive(Archive)]` will create an acceptable
/// implementation. You can use the `#[archive(...)]` attribute to control how
/// the implementation is generated. See the [`Archive`](macro@Archive) derive
/// macro for more details.
///
/// ```
/// use rkyv::{
///     archived_value,
///     ser::{Serializer, serializers::WriteSerializer},
///     Archive,
///     Archived,
///     Serialize,
/// };
///
/// #[derive(Archive, Serialize)]
/// struct Test {
///     int: u8,
///     string: String,
///     option: Option<Vec<i32>>,
/// }
///
/// let value = Test {
///     int: 42,
///     string: "hello world".to_string(),
///     option: Some(vec![1, 2, 3, 4]),
/// };
///
/// let mut serializer = WriteSerializer::new(Vec::new());
/// let pos = serializer.archive(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
///
/// let archived = unsafe { archived_value::<Test>(buf.as_slice(), pos) };
/// assert_eq!(archived.int, value.int);
/// assert_eq!(archived.string, value.string);
/// assert_eq!(archived.option, value.option);
/// ```
///
/// Many of the core and standard library types already have Archive
/// implementations available, but you may need to implement `Archive` for your
/// own types in some cases the derive macro cannot handle.
///
/// In this example, we add our own wrapper that serializes a `&'static str` as
/// if it's owned. Normally you can lean on the archived version of `String` to
/// do most of the work, but this example does everything to demonstrate how to
/// implement `Archive` for your own types.
///
/// ```
/// use core::{slice, str};
/// use rkyv::{
///     archived_value,
///     offset_of,
///     ser::{Serializer, serializers::WriteSerializer},
///     Archive,
///     Archived,
///     RelPtr,
///     Serialize,
/// };
///
/// struct OwnedStr {
///     inner: &'static str,
/// }
///
/// struct ArchivedOwnedStr {
///     // This will be a relative pointer to the bytes of our string.
///     ptr: RelPtr,
///     // The length of the archived version must be explicitly sized for
///     // 32/64-bit compatibility. Archive is not implemented for usize and
///     // isize to help you avoid making this mistake.
///     len: u32,
/// }
///
/// impl ArchivedOwnedStr {
///     // This will help us get the bytes of our type as a str again.
///     fn as_str(&self) -> &str {
///         unsafe {
///             // The as_ptr() function of RelPtr will get a pointer
///             // to its memory.
///             let bytes = slice::from_raw_parts(self.ptr.as_ptr(), self.len as usize);
///             str::from_utf8_unchecked(bytes)
///         }
///     }
/// }
///
/// struct OwnedStrResolver {
///     // This will be the position that the bytes of our string are stored at.
///     // We'll use this to make the relative pointer of our ArchivedOwnedStr.
///     bytes_pos: usize,
/// }
///
/// impl Archive for OwnedStr {
///     type Archived = ArchivedOwnedStr;
///     /// This is the resolver we'll return from archive.
///     type Resolver = OwnedStrResolver;
///
///     // The resolve function consumes the resolver and produces the archived
///     // value at the given position.
///     fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived {
///         Self::Archived {
///             // We have to be careful to add the offset of the ptr field,
///             // otherwise we'll be using the position of the ArchivedOwnedStr
///             // instead of the position of the ptr. That's the reason why
///             // RelPtr::new is unsafe.
///             ptr: unsafe {
///                 RelPtr::new(pos + offset_of!(ArchivedOwnedStr, ptr), resolver.bytes_pos)
///             },
///             len: self.inner.len() as u32,
///         }
///     }
/// }
///
/// impl<S: Serializer + ?Sized> Serialize<S> for OwnedStr {
///     fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
///         // This is where we want to write the bytes of our string and return
///         // a resolver that knows where those bytes were written.
///         let bytes_pos = serializer.pos();
///         serializer.write(self.inner.as_bytes())?;
///         Ok(Self::Resolver { bytes_pos })
///     }
/// }
///
/// let mut serializer = WriteSerializer::new(Vec::new());
/// const STR_VAL: &'static str = "I'm in an OwnedStr!";
/// let value = OwnedStr { inner: STR_VAL };
/// // It works!
/// let pos = serializer.archive(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = unsafe { archived_value::<OwnedStr>(buf.as_ref(), pos) };
/// // Let's make sure our data got written correctly
/// assert_eq!(archived.as_str(), STR_VAL);
/// ```
pub trait Archive {
    /// The archived version of this type.
    type Archived;

    /// The resolver for this type. It must contain all the information needed
    /// to make the archived type from the normal type.
    type Resolver;

    /// Creates the archived version of the given value at the given position.
    fn resolve(&self, pos: usize, resolver: Self::Resolver) -> Self::Archived;
}

pub trait Serialize<S: Fallible + ?Sized>: Archive {
    /// Writes the dependencies for the object and returns a resolver that can
    /// create the archived type.
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error>;
}

pub trait Fallible {
    type Error: 'static;
}

/// Converts a type back from its archived form.
///
/// This can be derived with [`Deserialize`](macro@Deserialize).
///
/// ## Examples
///
/// ```
/// use rkyv::{
///     archived_value,
///     de::deserializers::AllocDeserializer,
///     ser::{Serializer, serializers::WriteSerializer},
///     Archive,
///     Archived,
///     Deserialize,
///     Serialize,
/// };
///
/// #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
/// struct Test {
///     int: u8,
///     string: String,
///     option: Option<Vec<i32>>,
/// }
///
/// let mut serializer = WriteSerializer::new(Vec::new());
/// let value = Test {
///     int: 42,
///     string: "hello world".to_string(),
///     option: Some(vec![1, 2, 3, 4]),
/// };
/// let pos = serializer.archive(&value)
///     .expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = unsafe { archived_value::<Test>(buf.as_ref(), pos) };
///
/// let deserialized = archived.deserialize(&mut AllocDeserializer).unwrap();
/// assert_eq!(value, deserialized);
/// ```
pub trait Deserialize<T: Archive<Archived = Self>, D: Fallible + ?Sized> {
    fn deserialize(&self, deserializer: &mut D) -> Result<T, D::Error>;
}

/// A counterpart of [`Deserialize`] that's suitable for unsized types.
pub trait DeserializeRef<T: ArchiveRef<Reference = Self> + ?Sized, D: Fallible + ?Sized>:
    Deref<Target = T::Archived> + DerefMut<Target = T::Archived> + Sized
{
    /// Deserializes a reference to the given value.
    ///
    /// # Safety
    ///
    /// The return value must be allocated using the given allocator function.
    unsafe fn deserialize_ref(&self, deserializer: &mut D) -> Result<*mut T, D::Error>;
}

/// This trait is a counterpart of [`Archive`] that's suitable for unsized
/// types.
///
/// Instead of archiving its value directly, `ArchiveRef` archives a type that
/// dereferences to its archived type. As a consequence, its resolver must be
/// `usize`.
///
/// `ArchiveRef` is automatically implemented for all types that implement
/// [`Archive`], and uses a [`RelPtr`] as the reference type.
///
/// `ArchiveRef` is already implemented for slices and string slices. Use the
/// `rkyv_dyn` crate to archive trait objects. Unfortunately, you'll have to
/// manually implement `ArchiveRef` for your other unsized types.
pub trait ArchiveRef {
    type Archived: ?Sized;

    type Reference: Deref<Target = Self::Archived> + DerefMut<Target = Self::Archived>;

    fn resolve_ref(&self, pos: usize, resolver: usize) -> Self::Reference;
}

pub trait SerializeRef<S: Fallible + ?Sized>: ArchiveRef {
    /// Writes the object and returns a resolver that can create the reference
    /// to the archived type.
    fn serialize_ref(&self, serializer: &mut S) -> Result<usize, S::Error>;
}

/// A trait that indicates that some [`Archive`] type can be copied directly to
/// an archive without additional processing.
///
/// Types that implement `ArchiveCopy` are not guaranteed to have `archive`
/// called on them to archive their value.
///
/// You can derive an implementation of `ArchiveCopy` by adding
/// `#[archive(copy)]` to the struct or enum. Types that implement `ArchiveCopy`
/// must also implement [`Copy`](core::marker::Copy).
///
/// `ArchiveCopy` must be manually implemented even if a type implements
/// [`Archive`] and [`Copy`](core::marker::Copy) because some types may
/// transform their data when writing to an archive.
///
/// ## Examples
/// ```
/// use rkyv::{
///     archived_value,
///     ser::{Serializer, serializers::WriteSerializer},
///     Archive,
///     Serialize,
/// };
///
/// #[derive(Archive, Serialize, Clone, Copy, Debug, PartialEq)]
/// #[archive(copy)]
/// struct Vector4<T>(T, T, T, T);
///
/// let mut serializer = WriteSerializer::new(Vec::new());
/// let value = Vector4(1f32, 2f32, 3f32, 4f32);
/// let pos = serializer.archive(&value)
///     .expect("failed to archive Vector4");
/// let buf = serializer.into_inner();
/// let archived_value = unsafe { archived_value::<Vector4<f32>>(buf.as_ref(), pos) };
/// assert_eq!(&value, archived_value);
/// ```
pub unsafe trait ArchiveCopy: Archive<Archived = Self> + Copy {}

/// The type used for offsets in relative pointers.
#[cfg(not(feature = "long_rel_ptrs"))]
pub type Offset = i32;

/// The type used for offsets in relative pointers.
#[cfg(feature = "long_rel_ptrs")]
pub type Offset = i64;

/// A pointer which resolves to relative to its position in memory.
///
/// See [`Archive`] for an example of creating one.
#[repr(transparent)]
#[derive(Debug)]
pub struct RelPtr {
    offset: Offset,
    _phantom: PhantomPinned,
}

impl RelPtr {
    /// Creates a relative pointer from one position to another.
    ///
    /// # Safety
    ///
    /// `from` must be the position of the relative pointer and `to` must be the
    /// position of some valid memory.
    pub unsafe fn new(from: usize, to: usize) -> Self {
        Self {
            offset: (to as isize - from as isize) as Offset,
            _phantom: PhantomPinned,
        }
    }

    /// Gets the offset of the relative pointer.
    pub fn offset(&self) -> isize {
        self.offset as isize
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    pub fn as_ptr<T>(&self) -> *const T {
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .offset(self.offset as isize)
                .cast::<T>()
        }
    }

    /// Returns an unsafe mutable pointer to the memory address being pointed to
    /// by this relative pointer.
    pub fn as_mut_ptr<T>(&mut self) -> *mut T {
        unsafe {
            (self as *mut Self)
                .cast::<u8>()
                .offset(self.offset as isize)
                .cast::<T>()
        }
    }
}

/// Alias for the archived version of some [`Archive`] type.
pub type Archived<T> = <T as Archive>::Archived;
/// Alias for the resolver for some [`Archive`] type.
pub type Resolver<T> = <T as Archive>::Resolver;

/// Wraps a type and aligns it to at least 16 bytes. Mainly used to align byte
/// buffers for [`BufferSerializer`].
///
/// ## Examples
/// ```
/// use core::mem;
/// use rkyv::Aligned;
///
/// assert_eq!(mem::align_of::<u8>(), 1);
/// assert_eq!(mem::align_of::<Aligned<u8>>(), 16);
/// ```
#[derive(Clone, Copy)]
#[repr(align(16))]
pub struct Aligned<T>(pub T);

impl<T: Deref> Deref for Aligned<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<T: DerefMut> DerefMut for Aligned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

impl<T: AsRef<[U]>, U> AsRef<[U]> for Aligned<T> {
    fn as_ref(&self) -> &[U] {
        self.0.as_ref()
    }
}

impl<T: AsMut<[U]>, U> AsMut<[U]> for Aligned<T> {
    fn as_mut(&mut self) -> &mut [U] {
        self.0.as_mut()
    }
}

/// Casts an archived value from the given byte array at the given position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the value is archived at the given position in
/// the byte array.
#[inline]
pub unsafe fn archived_value<T: Archive + ?Sized>(bytes: &[u8], pos: usize) -> &T::Archived {
    &*bytes.as_ptr().add(pos).cast()
}

/// Casts a mutable archived value from the given byte array at the given
/// position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the value is archived at the given position in
/// the byte array.
#[inline]
pub unsafe fn archived_value_mut<T: Archive + ?Sized>(
    bytes: Pin<&mut [u8]>,
    pos: usize,
) -> Pin<&mut T::Archived> {
    Pin::new_unchecked(&mut *bytes.get_unchecked_mut().as_mut_ptr().add(pos).cast())
}

/// Casts an archived reference from the given byte array at the given position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the reference is archived at the given position
/// in the byte array.
#[inline]
pub unsafe fn archived_value_ref<T: ArchiveRef + ?Sized>(bytes: &[u8], pos: usize) -> &T::Reference {
    &*bytes.as_ptr().add(pos).cast()
}

/// Casts a mutable archived reference from the given byte array at the given
/// position.
///
/// This helps avoid situations where lifetimes get inappropriately assigned and
/// allow buffer mutation after getting archived value references.
///
/// # Safety
///
/// This is only safe to call if the reference is archived at the given position
/// in the byte array.
#[inline]
pub unsafe fn archived_value_ref_mut<T: ArchiveRef + ?Sized>(
    bytes: Pin<&mut [u8]>,
    pos: usize,
) -> Pin<&mut T::Reference> {
    Pin::new_unchecked(&mut *bytes.get_unchecked_mut().as_mut_ptr().add(pos).cast())
}
