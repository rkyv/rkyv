//! # rkyv
//!
//! rkyv (*archive*) is a zero-copy deserialization framework for Rust.
//!
//! It's similar to other zero-copy deserialization frameworks such as
//! [Cap'n Proto](https://capnproto.org) and [FlatBuffers](https://google.github.io/flatbuffers).
//! However, while the former have external schemas and heavily restricted data types, rkyv allows
//! all serialized types to be defined in code and can serialize a wide variety of types that the
//! others cannot. Additionally, rkyv is designed to have little to no overhead, and in most cases
//! will perform exactly the same as native types.
//!
//! ## Design
//!
//! Like [serde](https://serde.rs), rkyv uses Rust's powerful trait system to serialize data without
//! the need for reflection. Despite having a wide array of features, you also only pay for what you
//! use. If your data checks out, the serialization process can be as simple as a `memcpy`! Like
//! serde, this allows rkyv to perform at speeds similar to handwritten serializers.
//!
//! Unlike serde, rkyv produces data that is guaranteed deserialization free. If you wrote your data
//! to disk, you can just `mmap` your file into memory, cast a pointer, and your data is ready to
//! use. This makes it ideal for high-performance and IO-bound applications.
//!
//! Limited data mutation is supported through `Pin` APIs, and archived values can be truly
//! deserialized with [`Deserialize`] if full mutation capabilities are needed.
//!
//! ## Type support
//!
//! rkyv has a hashmap implementation that is built for zero-copy deserialization, so you can
//! serialize your hashmaps with abandon. The implementation performs perfect hashing with the
//! compress, hash and displace algorithm to use as little memory as possible while still performing
//! fast lookups.
//!
//! rkyv also has support for contextual serialization, deserialization, and validation. It can
//! properly serialize and deserialize shared pointers like `Rc` and `Arc`, and can be extended to
//! support custom contextual types.
//!
//! One of the most impactful features made possible by rkyv is the ability to serialize trait
//! objects and use them *as trait objects* without deserialization. See the `archive_dyn` crate for
//! more details.
//!
//! ## Tradeoffs
//!
//! rkyv is designed primarily for loading bulk game data as efficiently as possible. While rkyv is
//! a great format for final data, it lacks a full schema system and isn't well equipped for data
//! migration. Using a serialization library like serde can help fill these gaps, and you can use
//! serde with the same types as rkyv conflict-free.
//!
//! ## Features
//!
//! - `const_generics`: Improves the trait implementations for arrays with support for all lengths
//!   (enabled by default)
//! - `size_64`: Archives `*size` as `*64` instead of `*32`. This is for large archive support
//! - `specialization`: Enables support for the unstable specialization feature for increased
//!   performance for a few specific cases
//! - `std`: Enables standard library support (enabled by default)
//! - `strict`: Guarantees that types will have the same representations across platforms and
//!   compilations. This is already the case in practice, but this feature provides a guarantee. It
//!   additionally provides C type compatibility.
//! - `validation`: Enables validation support through `bytecheck`
//!
//! ## Examples
//!
//! See [`Archive`] for examples of how to use rkyv.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "specialization", feature(min_specialization))]
#![cfg_attr(feature = "specialization", feature(rustc_attrs))]

#[macro_use]
pub mod macros;
pub mod core_impl;
pub mod de;
pub mod ser;
#[cfg(feature = "std")]
pub mod std_impl;
pub mod util;
#[cfg(feature = "validation")]
pub mod validation;

use core::{
    fmt,
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
};

pub use memoffset::offset_of;
use ptr_meta::Pointee;
pub use rkyv_derive::{Archive, Deserialize, Serialize};
pub use util::*;
#[cfg(feature = "validation")]
pub use validation::{check_archived_root, check_archived_value};

/// Contains the error type for traits with methods that can fail
pub trait Fallible {
    /// The error produced by any failing methods
    type Error: 'static;
}

/// An error that can never be produced
#[derive(Debug)]
pub enum Unreachable {}

impl fmt::Display for Unreachable {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Unreachable {}

/// A fallible type that cannot produce errors
#[derive(Debug)]
pub struct Infallible;

impl Fallible for Infallible {
    type Error = Unreachable;
}

/// A type that can be used without deserializing.
///
/// Archiving is done depth-first, writing any data owned by a type before writing the data for the
/// type itself. The type must be able to create the archived type from only its own data and its
/// resolver.
///
/// ## Examples
///
/// Most of the time, `#[derive(Archive)]` will create an acceptable implementation. You can use the
/// `#[archive(...)]` attribute to control how the implementation is generated. See the
/// [`Archive`](macro@Archive) derive macro for more details.
///
/// ```
/// use rkyv::{
///     archived_root,
///     de::deserializers::AllocDeserializer,
///     ser::{Serializer, serializers::AlignedSerializer},
///     AlignedVec,
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
/// let value = Test {
///     int: 42,
///     string: "hello world".to_string(),
///     option: Some(vec![1, 2, 3, 4]),
/// };
///
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// serializer.serialize_value(&value).expect("failed to archive test");
/// let buf = serializer.into_inner();
///
/// let archived = unsafe { archived_root::<Test>(buf.as_slice()) };
/// assert_eq!(archived.int, value.int);
/// assert_eq!(archived.string, value.string);
/// assert_eq!(archived.option, value.option);
///
/// let deserialized = archived.deserialize(&mut AllocDeserializer).unwrap();
/// assert_eq!(value, deserialized);
/// ```
///
/// Many of the core and standard library types already have `Archive` implementations available,
/// but you may need to implement `Archive` for your own types in some cases the derive macro cannot
/// handle.
///
/// In this example, we add our own wrapper that serializes a `&'static str` as if it's owned.
/// Normally you can lean on the archived version of `String` to do most of the work, but this
/// example does everything to demonstrate how to implement `Archive` for your own types.
///
/// ```
/// use core::{mem::MaybeUninit, slice, str};
/// use rkyv::{
///     archived_root,
///     offset_of,
///     project_struct,
///     ser::{Serializer, serializers::AlignedSerializer},
///     AlignedVec,
///     Archive,
///     Archived,
///     ArchiveUnsized,
///     MetadataResolver,
///     RelPtr,
///     Serialize,
///     SerializeUnsized,
/// };
///
/// struct OwnedStr {
///     inner: &'static str,
/// }
///
/// struct ArchivedOwnedStr {
///     // This will be a relative pointer to our string
///     ptr: RelPtr<str>,
/// }
///
/// impl ArchivedOwnedStr {
///     // This will help us get the bytes of our type as a str again.
///     fn as_str(&self) -> &str {
///         unsafe {
///             // The as_ptr() function of RelPtr will get a pointer the str
///             &*self.ptr.as_ptr()
///         }
///     }
/// }
///
/// struct OwnedStrResolver {
///     // This will be the position that the bytes of our string are stored at.
///     // We'll use this to resolve the relative pointer of our
///     // ArchivedOwnedStr.
///     pos: usize,
///     // The archived metadata for our str may also need a resolver.
///     metadata_resolver: MetadataResolver<str>,
/// }
///
/// // The Archive implementation defines the archived version of our type and
/// // determines how to turn the resolver into the archived form. The Serialize
/// // implementations determine how to make a resolver from the original value.
/// impl Archive for OwnedStr {
///     type Archived = ArchivedOwnedStr;
///     // This is the resolver we can create our Archived verison from.
///     type Resolver = OwnedStrResolver;
///
///     // The resolve function consumes the resolver and produces the archived
///     // value at the given position.
///     fn resolve(
///         &self,
///         pos: usize,
///         resolver: Self::Resolver,
///         out: &mut MaybeUninit<Self::Archived>
///     ) {
///         // We have to be careful to add the offset of the ptr field,
///         // otherwise we'll be using the position of the ArchivedOwnedStr
///         // instead of the position of the relative pointer.
///         unsafe {
///             self.inner.resolve_unsized(
///                 pos + offset_of!(Self::Archived, ptr),
///                 resolver.pos,
///                 resolver.metadata_resolver,
///                 project_struct!(out: Self::Archived => ptr),
///             );
///         }
///     }
/// }
///
/// // We restrict our serializer types with Serializer because we need its
/// // capabilities to archive our type. For other types, we might need more or
/// // less restrictive bounds on the type of S.
/// impl<S: Serializer + ?Sized> Serialize<S> for OwnedStr {
///     fn serialize(
///         &self,
///         serializer: &mut S
///     ) -> Result<Self::Resolver, S::Error> {
///         // This is where we want to write the bytes of our string and return
///         // a resolver that knows where those bytes were written.
///         // We also need to serialize the metadata for our str.
///         Ok(OwnedStrResolver {
///             pos: self.inner.serialize_unsized(serializer)?,
///             metadata_resolver: self.inner.serialize_metadata(serializer)?
///         })
///     }
/// }
///
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// const STR_VAL: &'static str = "I'm in an OwnedStr!";
/// let value = OwnedStr { inner: STR_VAL };
/// // It works!
/// serializer.serialize_value(&value).expect("failed to archive test");
/// let buf = serializer.into_inner();
/// let archived = unsafe { archived_root::<OwnedStr>(buf.as_ref()) };
/// // Let's make sure our data got written correctly
/// assert_eq!(archived.as_str(), STR_VAL);
/// ```
pub trait Archive {
    /// The archived version of this type.
    type Archived;

    /// The resolver for this type. It must contain all the information needed to make the archived
    /// type from the normal type.
    type Resolver;

    /// Creates the archived version of this value at the given position and writes it to the given
    /// output.
    ///
    /// The output should be initialized field-by-field rather than by writing a whole struct. This
    /// is because performing a typed copy will set all of the padding bytes to uninitialized, but
    /// they must remain whatever value they currently have. This is so that uninitialized memory
    /// doesn't get leaked to the final archive.
    fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>);
}

/// Converts a type to its archived form.
///
/// See [`Archive`] for examples of implementing `Serialize`.
pub trait Serialize<S: Fallible + ?Sized>: Archive {
    /// Writes the dependencies for the object and returns a resolver that can create the archived
    /// type.
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error>;
}

/// Converts a type back from its archived form.
///
/// This can be derived with [`Deserialize`](macro@Deserialize).
pub trait Deserialize<T: Archive<Archived = Self>, D: Fallible + ?Sized> {
    /// Deserializes using the given deserializer
    fn deserialize(&self, deserializer: &mut D) -> Result<T, D::Error>;
}

/// A counterpart of [`Archive`] that's suitable for unsized types.
///
/// Instead of archiving its value directly, `ArchiveUnsized` archives a [`RelPtr`] to its archived
/// type. As a consequence, its resolver must be `usize`.
///
/// `ArchiveUnsized` is automatically implemented for all types that implement [`Archive`].
///
/// `ArchiveUnsized` is already implemented for slices and string slices, and the `rkyv_dyn` crate
/// can be used to archive trait objects. Other unsized types must manually implement
/// `ArchiveUnsized`.
///
/// ## Examples
///
/// ```
/// use core::{
///     mem::{transmute, MaybeUninit},
///     ops::{Deref, DerefMut},
/// };
/// use ptr_meta::Pointee;
/// use rkyv::{
///     archived_unsized_value,
///     offset_of,
///     ser::{serializers::AlignedSerializer, Serializer},
///     AlignedVec,
///     Archive,
///     Archived,
///     ArchivedMetadata,
///     ArchivedUsize,
///     ArchivePointee,
///     ArchiveUnsized,
///     RelPtr,
///     Serialize,
///     SerializeUnsized,
/// };
///
/// // We're going to be dealing mostly with blocks that have a trailing slice
/// pub struct Block<H, T: ?Sized> {
///     head: H,
///     tail: T,
/// }
///
/// impl<H, T> Pointee for Block<H, [T]> {
///     type Metadata = usize;
/// }
///
/// // For blocks with trailing slices, we need to store the length of the slice
/// // in the metadata.
/// pub struct BlockSliceMetadata {
///     len: ArchivedUsize,
/// }
///
/// // ArchivePointee is automatically derived for sized types because pointers
/// // to sized types don't need to store any extra information. Because we're
/// // making an unsized block, we need to define what metadata gets stored with
/// // our data pointer.
/// impl<H, T> ArchivePointee for Block<H, [T]> {
///     // This is the extra data that needs to get stored for blocks with
///     // trailing slices
///     type ArchivedMetadata = BlockSliceMetadata;
///
///     // We need to be able to turn our archived metadata into regular
///     // metadata for our type
///     fn pointer_metadata(
///         archived: &Self::ArchivedMetadata
///     ) -> <Self as Pointee>::Metadata {
///         archived.len as usize
///     }
/// }
///
/// // We're implementing ArchiveUnsized for just Block<H, [T]>. We can still
/// // implement Archive for blocks with sized tails and they won't conflict.
/// impl<H: Archive, T: Archive> ArchiveUnsized for Block<H, [T]> {
///     // We'll reuse our block type as our archived type.
///     type Archived = Block<Archived<H>, [Archived<T>]>;
///
///     // This is where we'd put any resolve data for our metadata.
///     // Most of the time, this can just be () because most metadata is Copy,
///     // but the option is there if you need it.
///     type MetadataResolver = ();
///
///     // Here's where we make the metadata for our pointer.
///     // This also gets the position and resolver for the metadata, but we
///     // don't need it in this case.
///     fn resolve_metadata(
///         &self,
///         _: usize,
///         _: Self::MetadataResolver,
///         out: &mut MaybeUninit<ArchivedMetadata<Self>>,
///     ) {
///         unsafe {
///             out.as_mut_ptr().write(BlockSliceMetadata {
///                 len: self.tail.len() as ArchivedUsize,
///             });
///         }
///     }
/// }
///
/// // The bounds we use on our serializer type indicate that we need basic
/// // serializer capabilities, and then whatever capabilities our head and tail
/// // types need to serialize themselves.
/// impl<
///     H: Serialize<S>,
///     T: Serialize<S>,
///     S: Serializer + ?Sized
/// > SerializeUnsized<S> for Block<H, [T]> {
///     // This is where we construct our unsized type in the serializer
///     fn serialize_unsized(
///         &self,
///         serializer: &mut S
///     ) -> Result<usize, S::Error> {
///         // First, we archive the head and all the tails. This will make sure
///         // that when we finally build our block, we don't accidentally mess
///         // up the structure with serialized dependencies.
///         let head_resolver = self.head.serialize(serializer)?;
///         let mut resolvers = Vec::new();
///         for tail in self.tail.iter() {
///             resolvers.push(tail.serialize(serializer)?);
///         }
///         // Now we align our serializer for our archived type and write it.
///         // We can't align for unsized types so we treat the trailing slice
///         // like an array of 0 length for now.
///         serializer.align_for::<Block<Archived<H>, [Archived<T>; 0]>>()?;
///         let result = unsafe {
///             serializer.resolve_aligned(&self.head, head_resolver)?
///         };
///         serializer.align_for::<Archived<T>>()?;
///         for (item, resolver) in self.tail.iter().zip(resolvers.drain(..)) {
///             unsafe {
///                 serializer.resolve_aligned(item, resolver)?;
///             }
///         }
///         Ok(result)
///     }
///
///     // This is where we serialize the metadata for our type. In this case,
///     // we do all the work in resolve and don't need to do anything here.
///     fn serialize_metadata(
///         &self,
///         serializer: &mut S
///     ) -> Result<Self::MetadataResolver, S::Error> {
///         Ok(())
///     }
/// }
///
/// let value = Block {
///     head: "Numbers 1-4".to_string(),
///     tail: [1, 2, 3, 4],
/// };
/// // We have a Block<String, [i32; 4]> but we want to it to be a
/// // Block<String, [i32]>, so we need to do more pointer transmutation
/// let ptr = (&value as *const Block<String, [i32; 4]>).cast::<()>();
/// let unsized_value = unsafe {
///     &*transmute::<(*const (), usize), *const Block<String, [i32]>>((ptr, 4))
/// };
///
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// let pos = serializer.serialize_unsized_value(unsized_value)
///     .expect("failed to archive block");
/// let buf = serializer.into_inner();
///
/// let archived_ref = unsafe {
///     archived_unsized_value::<Block<String, [i32]>>(buf.as_slice(), pos)
/// };
/// assert_eq!(archived_ref.head, "Numbers 1-4");
/// assert_eq!(archived_ref.tail.len(), 4);
/// assert_eq!(archived_ref.tail, [1, 2, 3, 4]);
/// ```
pub trait ArchiveUnsized: Pointee {
    /// The archived counterpart of this type. Unlike `Archive`, it may be unsized.
    type Archived: ArchivePointee + ?Sized;

    /// The resolver for the metadata of this type.
    type MetadataResolver;

    /// Creates the archived version of the metadata for this value at the given position and writes
    /// it to the given output.
    ///
    /// The output should be initialized field-by-field rather than by writing a whole struct. This
    /// is because performing a typed copy will set all of the padding bytes to uninitialized, but
    /// they must remain whatever value they currently have. This is so that uninitialized memory
    /// doesn't get leaked to the final archive.
    fn resolve_metadata(
        &self,
        pos: usize,
        resolver: Self::MetadataResolver,
        out: &mut MaybeUninit<ArchivedMetadata<Self>>,
    );

    /// Resolves a relative pointer to this value with the given `from` and `to` and writes it to
    /// the given output.
    #[inline]
    fn resolve_unsized(
        &self,
        from: usize,
        to: usize,
        resolver: Self::MetadataResolver,
        out: &mut MaybeUninit<RelPtr<Self::Archived>>,
    ) {
        RelPtr::resolve_emplace(from, to, self, resolver, out);
    }
}

/// An archived type with associated metadata for its relative pointer.
///
/// This is mostly used in the context of smart pointers and unsized types, and is implemented for
/// all sized types by default.
pub trait ArchivePointee: Pointee {
    /// The archived version of the pointer metadata for this type.
    type ArchivedMetadata;

    /// Converts some archived metadata to the pointer metadata for itself.
    fn pointer_metadata(archived: &Self::ArchivedMetadata) -> <Self as Pointee>::Metadata;
}

/// A counterpart of [`Serialize`] that's suitable for unsized types.
///
/// See [`ArchiveUnsized`] for examples of implementing `SerializeUnsized`.
pub trait SerializeUnsized<S: Fallible + ?Sized>: ArchiveUnsized {
    /// Writes the object and returns the position of the archived type.
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error>;

    /// Serializes the metadata for the given type.
    fn serialize_metadata(&self, serializer: &mut S) -> Result<Self::MetadataResolver, S::Error>;
}

/// A counterpart of [`Deserialize`] that's suitable for unsized types.
///
/// Most types that implement `DeserializeUnsized` will need a [`Deserializer`](de::Deserializer)
/// bound so that they can allocate memory.
pub trait DeserializeUnsized<T: ArchiveUnsized<Archived = Self> + ?Sized, D: Fallible + ?Sized>:
    ArchivePointee
{
    /// Deserializes a reference to the given value.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that the memory returned is properly deallocated.
    unsafe fn deserialize_unsized(&self, deserializer: &mut D) -> Result<*mut (), D::Error>;

    /// Deserializes the metadata for the given type.
    fn deserialize_metadata(&self, deserializer: &mut D) -> Result<T::Metadata, D::Error>;
}

/// An [`Archive`] type that is a bitwise copy of itself and without additional processing.
///
/// Types that implement `ArchiveCopy` are not guaranteed to have a [`Serialize`] implementation
/// called on them to archive their value.
///
/// You can derive an implementation of `ArchiveCopy` by adding `#[archive(copy)]` to the struct or
/// enum. Types that implement `ArchiveCopy` must also implement [`Copy`](core::marker::Copy).
///
/// `ArchiveCopy` must be manually implemented even if a type implements [`Archive`] and
/// [`Copy`](core::marker::Copy) because some types may transform their data when writing to an
/// archive.
///
/// ## Examples
/// ```
/// use rkyv::{
///     archived_root,
///     ser::{Serializer, serializers::AlignedSerializer},
///     AlignedVec,
///     Archive,
///     Serialize,
/// };
///
/// #[derive(Archive, Serialize, Clone, Copy, Debug, PartialEq)]
/// #[archive(copy)]
/// struct Vector4<T>(T, T, T, T);
///
/// let mut serializer = AlignedSerializer::new(AlignedVec::new());
/// let value = Vector4(1f32, 2f32, 3f32, 4f32);
/// serializer.serialize_value(&value).expect("failed to archive Vector4");
/// let buf = serializer.into_inner();
/// let archived_value = unsafe { archived_root::<Vector4<f32>>(buf.as_ref()) };
/// assert_eq!(&value, archived_value);
/// ```
#[cfg_attr(feature = "specialization", rustc_unsafe_specialization_marker)]
pub unsafe trait ArchiveCopy: Archive<Archived = Self> + Copy {}

/// The type used for sizes in archived types.
#[cfg(not(feature = "size_64"))]
pub type ArchivedUsize = u32;

/// The type used for offsets in relative pointers.
#[cfg(not(feature = "size_64"))]
pub type ArchivedIsize = i32;

/// The type used for sizes in archived types.
#[cfg(feature = "size_64")]
pub type ArchivedUsize = u64;

/// The type used for offsets in relative pointers.
#[cfg(feature = "size_64")]
pub type ArchivedIsize = i64;

/// An untyped pointer which resolves relative to its position in memory.
#[derive(Debug)]
#[repr(transparent)]
pub struct RawRelPtr {
    offset: ArchivedIsize,
    _phantom: PhantomPinned,
}

impl RawRelPtr {
    /// Emplaces a new relative pointer between the given positions and stores it in the given
    /// output.
    #[inline]
    pub fn emplace(from: usize, to: usize, out: &mut MaybeUninit<Self>) {
        let offset = (to as isize - from as isize) as ArchivedIsize;
        unsafe {
            project_struct!(out: Self => offset: ArchivedIsize)
                .as_mut_ptr()
                .write(offset);
        }
    }

    /// Creates a new relative pointer that has an offset of 0.
    #[inline]
    pub fn null() -> Self {
        Self {
            offset: 0,
            _phantom: PhantomPinned,
        }
    }

    /// Checks whether the relative pointer is null.
    #[inline]
    pub fn is_null(&self) -> bool {
        self.offset == 0
    }

    /// Gets the base pointer for the relative pointer.
    #[inline]
    pub fn base(&self) -> *const u8 {
        (self as *const Self).cast::<u8>()
    }

    /// Gets the offset of the relative pointer.
    #[inline]
    pub fn offset(&self) -> isize {
        self.offset as isize
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    #[inline]
    pub fn as_ptr(&self) -> *const () {
        unsafe {
            (self as *const Self)
                .cast::<u8>()
                .offset(self.offset as isize)
                .cast()
        }
    }

    /// Returns an unsafe mutable pointer to the memory address being pointed to
    /// by this relative pointer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut () {
        unsafe {
            (self as *mut Self)
                .cast::<u8>()
                .offset(self.offset as isize)
                .cast()
        }
    }
}

/// A pointer which resolves to relative to its position in memory.
///
/// See [`Archive`] for an example of creating one.
#[cfg_attr(feature = "strict", repr(C))]
pub struct RelPtr<T: ArchivePointee + ?Sized> {
    raw_ptr: RawRelPtr,
    metadata: T::ArchivedMetadata,
    _phantom: PhantomData<T>,
}

impl<T: ArchivePointee + ?Sized> RelPtr<T> {
    /// Creates a new relative pointer from the given raw pointer and metadata.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that:
    /// - `raw_ptr` is a valid relative pointer in its final position
    /// - `raw_ptr` points to a valid value
    /// - `metadata` is valid metadata for the pointed value.
    #[inline]
    pub fn new(raw_ptr: RawRelPtr, metadata: T::ArchivedMetadata) -> Self {
        Self {
            raw_ptr,
            metadata,
            _phantom: PhantomData,
        }
    }

    /// Creates a relative pointer from one position to another.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `from` is the position of the relative pointer and `to` is
    /// the position of some valid memory.
    #[inline]
    pub fn resolve_emplace<U: ArchiveUnsized<Archived = T> + ?Sized>(
        from: usize,
        to: usize,
        value: &U,
        metadata_resolver: U::MetadataResolver,
        out: &mut MaybeUninit<Self>,
    ) {
        RawRelPtr::emplace(
            from + offset_of!(Self, raw_ptr),
            to,
            project_struct!(out: Self => raw_ptr),
        );
        value.resolve_metadata(
            from + offset_of!(Self, metadata),
            metadata_resolver,
            project_struct!(out: Self => metadata),
        );
    }

    /// Gets the base pointer for the relative pointer.
    #[inline]
    pub fn base(&self) -> *const u8 {
        self.raw_ptr.base()
    }

    /// Gets the offset of the relative pointer.
    #[inline]
    pub fn offset(&self) -> isize {
        self.raw_ptr.offset()
    }

    /// Gets the metadata of the relative pointer.
    #[inline]
    pub fn metadata(&self) -> &T::ArchivedMetadata {
        &self.metadata
    }

    /// Calculates the memory address being pointed to by this relative pointer.
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        ptr_meta::from_raw_parts(self.raw_ptr.as_ptr(), T::pointer_metadata(&self.metadata))
    }

    /// Returns an unsafe mutable pointer to the memory address being pointed to by this relative
    /// pointer.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        ptr_meta::from_raw_parts_mut(
            self.raw_ptr.as_mut_ptr(),
            T::pointer_metadata(&self.metadata),
        )
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Debug for RelPtr<T>
where
    T::ArchivedMetadata: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelPtr")
            .field("raw_ptr", &self.raw_ptr)
            .field("metadata", &self.metadata)
            .field("_phantom", &self._phantom)
            .finish()
    }
}

/// Alias for the archived version of some [`Archive`] type.
pub type Archived<T> = <T as Archive>::Archived;
/// Alias for the resolver for some [`Archive`] type.
pub type Resolver<T> = <T as Archive>::Resolver;
/// Alias for the archived metadata for some [`ArchiveUnsized`] type.
pub type ArchivedMetadata<T> =
    <<T as ArchiveUnsized>::Archived as ArchivePointee>::ArchivedMetadata;
/// Alias for the metadata resolver for some [`ArchiveUnsized`] type.
pub type MetadataResolver<T> = <T as ArchiveUnsized>::MetadataResolver;
