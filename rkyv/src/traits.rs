//! The core traits provided by rkyv.

use core::{
    alloc::{Layout, LayoutError},
    hash::Hash,
    marker::PhantomData,
};

use crate::{
    place::Initialized,
    ptr_meta::Pointee,
    rancor::Fallible,
    ser::{Writer, WriterExt as _},
    ArchivedMetadata, Place, RelPtr,
};

/// A type with a stable, well-defined layout that is the same on all targets.
///
/// # Safety
///
/// To implement this trait, a type must have a stable, well-defined layout that
/// is the same on all targets. For structs and unions, this means that they
/// must be `#[repr(transparent)]` or `#[repr(C)]`. For enums, this means that
/// they must be `#[repr(C)]`, `#[repr(int)]`, or `#[repr(C, int)]`.
/// Additionally, all fields that the type may contain or produce relative
/// pointers to must also be `Portable`.
pub unsafe trait Portable {}

/// Returns the layout of a type from its metadata.
pub trait LayoutRaw
where
    Self: Pointee,
{
    /// Returns the layout of the type.
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError>;
}

/// An optimization hint about whether `T` is trivially copyable.
pub struct CopyOptimization<T: ?Sized>(bool, PhantomData<T>);

impl<T: ?Sized> CopyOptimization<T> {
    /// Returns a `TriviallyCopyable` hint with the optimization enabled for
    /// `T`.
    ///
    /// # Safety
    ///
    /// `T` must not have any uninit bytes (e.g. padding).
    pub const unsafe fn enable() -> Self {
        Self(true, PhantomData)
    }

    /// Returns a `TriviallyCopyable` hint with the optimization enabled for
    /// `T` if `value` is `true`.
    ///
    /// # Safety
    ///
    /// `T` must not have any uninit bytes (e.g. padding) if `value` is true.
    pub const unsafe fn enable_if(value: bool) -> Self {
        Self(value, PhantomData)
    }

    /// Returns a `TriviallyCopyable` hint with the optimization disabled for
    /// `T`.
    pub const fn disable() -> Self {
        Self(false, PhantomData)
    }

    /// Returns whether the optimization is enabled for `T`.
    pub const fn is_enabled(&self) -> bool {
        self.0
    }
}

/// A type that can be used without deserializing.
///
/// `Archive` is one of three basic traits used to work with zero-copy data and
/// controls the layout of the data in its archived zero-copy representation.
/// The [`Serialize`] trait helps transform types into that representation, and
/// the [`Deserialize`] trait helps transform types back out.
///
/// Types that implement `Archive` must have a well-defined archived size.
/// Unsized types can be supported using the [`ArchiveUnsized`] trait, along
/// with [`SerializeUnsized`] and [`DeserializeUnsized`].
///
/// Archiving is done depth-first, writing any data owned by a type before
/// writing the data for the type itself. The type must be able to create the
/// archived type from only its own data and its resolver.
///
/// Archived data is always treated as if it is tree-shaped, with the root
/// owning its direct descendents and so on. Data that is not tree-shaped can be
/// supported using special serializer and deserializer bounds (see
/// [`ArchivedRc`](crate::rc::ArchivedRc) for example). In a buffer of
/// serialized data, objects are laid out in *reverse order*. This means that
/// the root object is located near the end of the buffer and leaf objects are
/// located near the beginning.
///
/// # Examples
///
/// Most of the time, `#[derive(Archive)]` will create an acceptable
/// implementation. You can use the `#[archive(...)]` and `#[archive_attr(...)]`
/// attributes to control how the implementation is generated. See the
/// [`Archive`](macro@crate::Archive) derive macro for more details.
///
/// ```
/// use rkyv::{rancor::Error, Archive, Archived, Deserialize, Serialize};
///
/// #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
/// // This will generate a PartialEq impl between our unarchived and archived
/// // types
/// #[archive(compare(PartialEq))]
/// // We can pass attributes through to generated types with archive_attr
/// #[archive_attr(derive(Debug))]
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
/// // Serializing is as easy as a single function call
/// let bytes = rkyv::to_bytes::<Error>(&value).unwrap();
///
/// // Or you can customize your serialization for better performance
/// // and compatibility with #![no_std] environments
/// use rkyv::{
///     ser::{allocator::Arena, sharing::Share, Serializer},
///     util::{serialize_into, AlignedVec},
/// };
///
/// let mut arena = Arena::new();
/// let serializer = serialize_into::<_, Error>(
///     &value,
///     Serializer::new(AlignedVec::new(), arena.acquire(), Share::new()),
/// )
/// .unwrap();
/// let bytes = serializer.into_writer();
///
/// // You can use the safe API with the `bytecheck` feature enabled,
/// // or you can use the unsafe API (shown here) for maximum performance
/// let archived =
///     unsafe { rkyv::access_unchecked::<Archived<Test>>(&bytes[..]) };
/// assert_eq!(archived, &value);
///
/// // And you can always deserialize back to the original type
/// let deserialized =
///     rkyv::deserialize::<Test, _, Error>(archived, &mut ()).unwrap();
/// assert_eq!(deserialized, value);
/// ```
///
/// _Note: the safe API requires the `bytecheck` feature._
///
/// Many of the core and standard library types already have `Archive`
/// implementations available, but you may need to implement `Archive` for your
/// own types in some cases the derive macro cannot handle.
///
/// In this example, we add our own wrapper that serializes a `&'static str` as
/// if it's owned. Normally you can lean on the archived version of `String` to
/// do most of the work, or use the [`Inline`](crate::with::Inline) to do
/// exactly this. This example does everything to demonstrate how to implement
/// `Archive` for your own types.
///
/// ```
/// use core::{slice, str};
///
/// use rkyv::{
///     access_unchecked,
///     rancor::{Error, Fallible},
///     ser::Writer,
///     to_bytes,
///     util::AlignedVec,
///     Archive, ArchiveUnsized, Archived, Portable, RelPtr, Serialize,
///     SerializeUnsized, munge::munge, Place,
/// };
///
/// struct OwnedStr {
///     inner: &'static str,
/// }
///
/// #[derive(Portable)]
/// #[repr(transparent)]
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
/// }
///
/// // The Archive implementation defines the archived version of our type and
/// // determines how to turn the resolver into the archived form. The Serialize
/// // implementations determine how to make a resolver from the original value.
/// impl Archive for OwnedStr {
///     type Archived = ArchivedOwnedStr;
///     // This is the resolver we can create our Archived version from.
///     type Resolver = OwnedStrResolver;
///
///     // The resolve function consumes the resolver and produces the archived
///     // value at the given position.
///     fn resolve(
///         &self,
///         resolver: Self::Resolver,
///         out: Place<Self::Archived>,
///     ) {
///         munge!(let ArchivedOwnedStr { ptr } = out);
///         RelPtr::emplace_unsized(
///             resolver.pos,
///             self.inner.archived_metadata(),
///             ptr,
///         );
///     }
/// }
///
/// // We restrict our serializer types with Writer because we need its
/// // capabilities to serialize the inner string. For other types, we might
/// // need more or less restrictive bounds on the type of S.
/// impl<S: Fallible + Writer + ?Sized> Serialize<S> for OwnedStr {
///     fn serialize(
///         &self,
///         serializer: &mut S,
///     ) -> Result<Self::Resolver, S::Error> {
///         // This is where we want to write the bytes of our string and return
///         // a resolver that knows where those bytes were written.
///         // We also need to serialize the metadata for our str.
///         Ok(OwnedStrResolver {
///             pos: self.inner.serialize_unsized(serializer)?,
///         })
///     }
/// }
///
/// const STR_VAL: &'static str = "I'm in an OwnedStr!";
/// let value = OwnedStr { inner: STR_VAL };
/// // It works!
/// let buf = to_bytes::<Error>(&value).expect("failed to serialize");
/// let archived =
///     unsafe { access_unchecked::<ArchivedOwnedStr>(buf.as_ref()) };
/// // Let's make sure our data got written correctly
/// assert_eq!(archived.as_str(), STR_VAL);
/// ```
pub trait Archive {
    /// An optimization flag that allows the bytes of this type to be copied
    /// directly to a writer instead of calling `serialize`.
    ///
    /// This optimization is disabled by default. To enable this optimization,
    /// you must unsafely attest that `Self` is trivially copyable using
    /// [`CopyOptimization::enable`] or [`CopyOptimization::enable_if`].
    const COPY_OPTIMIZATION: CopyOptimization<Self> =
        CopyOptimization::disable();

    /// The archived representation of this type.
    ///
    /// In this form, the data can be used with zero-copy deserialization.
    type Archived: Portable;

    /// The resolver for this type. It must contain all the additional
    /// information from serializing needed to make the archived type from
    /// the normal type.
    type Resolver;

    /// Creates the archived version of this value at the given position and
    /// writes it to the given output.
    ///
    /// The output should be initialized field-by-field rather than by writing a
    /// whole struct. Performing a typed copy will mark all of the padding
    /// bytes as uninitialized, but they must remain set to the value they
    /// currently have. This prevents leaking uninitialized memory to
    /// the final archive.
    ///
    /// # Safety
    ///
    /// None
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>);
}

/// Converts a type to its archived form.
///
/// Objects perform any supportive serialization during
/// [`serialize`](Serialize::serialize). For types that reference nonlocal
/// (pointed-to) data, this is when that data must be serialized to the output.
/// These types will need to bound `S` to implement [`Writer`] and
/// any other required traits (e.g. [`Sharing`](crate::ser::Sharing)). They
/// should then serialize their dependencies during `serialize`.
///
/// See [`Archive`] for examples of implementing `Serialize`.
pub trait Serialize<S: Fallible + ?Sized>: Archive {
    /// Writes the dependencies for the object and returns a resolver that can
    /// create the archived type.
    fn serialize(&self, serializer: &mut S)
        -> Result<Self::Resolver, S::Error>;

    /// Archives the given object and returns the position it was archived at.
    #[inline]
    fn serialize_and_resolve(
        &self,
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        S: Writer,
    {
        let resolver = self.serialize(serializer)?;
        serializer.align_for::<Self::Archived>()?;
        unsafe { serializer.resolve_aligned(self, resolver) }
    }
}

/// Converts a type back from its archived form.
///
/// Some types may require specific deserializer capabilities, such as `Rc` and
/// `Arc`. In these cases, the deserializer type `D` should be bound so that it
/// implements traits that provide those capabilities (e.g.
/// [`Pooling`](crate::de::Pooling)).
///
/// This can be derived with [`Deserialize`](macro@crate::Deserialize).
pub trait Deserialize<T, D: Fallible + ?Sized> {
    /// Deserializes using the given deserializer
    fn deserialize(&self, deserializer: &mut D) -> Result<T, D::Error>;
}

/// A counterpart of [`Archive`] that's suitable for unsized types.
///
/// Unlike `Archive`, types that implement `ArchiveUnsized` must be serialized
/// separately from their owning object. For example, whereas an `i32` might be
/// laid out as part of a larger struct, a `Box<i32>` would serialize the `i32`
/// somewhere in the archive and the `Box` would point to it as part of the
/// larger struct. Because of this, the equivalent
/// [`Resolver`](Archive::Resolver) type for `ArchiveUnsized` is always a
/// `usize` representing the position of the serialized value.
///
/// `ArchiveUnsized` is automatically implemented for all types that implement
/// [`Archive`]. Nothing special needs to be done to use them with types like
/// `Box`, `Rc`, and `Arc`. It is also already implemented for slices and string
/// slices, and the `rkyv_dyn` crate can be used to archive trait objects. Other
/// unsized types must manually implement `ArchiveUnsized`.
///
/// # Examples
///
/// This example shows how to manually implement `ArchiveUnsized` for an unsized
/// type. Special care must be taken to ensure that the types are laid out
/// correctly.
///
/// ```
/// use core::{
///     mem::transmute,
///     ops::{Deref, DerefMut},
/// };
///
/// use ptr_meta::Pointee;
/// use rkyv::{
///     access_unchecked,
///     primitive::ArchivedUsize,
///     rancor::{Error, Fallible},
///     ser::{Positional, Writer, WriterExt as _},
///     to_bytes,
///     util::AlignedVec,
///     Archive, ArchivePointee, ArchiveUnsized, Archived, ArchivedMetadata,
///     Portable, RelPtr, Serialize, SerializeUnsized,
/// };
///
/// // We're going to be dealing mostly with blocks that have a trailing slice
/// #[derive(Portable)]
/// #[repr(C)]
/// pub struct Block<H, T: ?Sized> {
///     head: H,
///     tail: T,
/// }
///
/// unsafe impl<H, T> Pointee for Block<H, [T]> {
///     type Metadata = <[T] as Pointee>::Metadata;
/// }
///
/// // ArchivePointee is automatically derived for sized types because pointers
/// // to sized types don't need to store any extra information. Because we're
/// // making an unsized block, we need to define what metadata gets stored with
/// // our data pointer.
/// impl<H, T> ArchivePointee for Block<H, [T]> {
///     // This is the extra data that needs to get stored for blocks with
///     // trailing slices
///     type ArchivedMetadata = <[T] as ArchivePointee>::ArchivedMetadata;
///
///     // We need to be able to turn our archived metadata into regular
///     // metadata for our type
///     fn pointer_metadata(
///         metadata: &Self::ArchivedMetadata,
///     ) -> <Self as Pointee>::Metadata {
///         metadata.to_native() as usize
///     }
/// }
///
/// // We're implementing ArchiveUnsized for just Block<H, [T]>. We can still
/// // implement Archive for blocks with sized tails and they won't conflict.
/// impl<H: Archive, T: Archive> ArchiveUnsized for Block<H, [T]> {
///     // We'll reuse our block type as our archived type.
///     type Archived = Block<Archived<H>, [Archived<T>]>;
///
///     // Here's where we make the metadata for our archived type.
///     fn archived_metadata(&self) -> ArchivedMetadata<Self> {
///         // Because the metadata for our `ArchivedBlock` is the metadata of
///         // the trailing slice, we just need to return that archived
///         // metadata.
///         self.tail.archived_metadata()
///     }
/// }
///
/// // The bounds we use on our serializer type indicate that we need basic
/// // serializer capabilities, and then whatever capabilities our head and tail
/// // types need to serialize themselves.
/// impl<H, T, S> SerializeUnsized<S> for Block<H, [T]>
/// where
///     H: Serialize<S>,
///     T: Serialize<S>,
///     S: Fallible + Writer + ?Sized,
/// {
///     // This is where we construct our unsized type in the serializer
///     fn serialize_unsized(
///         &self,
///         serializer: &mut S,
///     ) -> Result<usize, S::Error> {
///         // First, we serialize the head and all the tails. This will make
///         // sure that when we finally build our block, we don't accidentally
///         // mess up the structure with serialized dependencies.
///         let head_resolver = self.head.serialize(serializer)?;
///         let mut resolvers = Vec::new();
///         for tail in self.tail.iter() {
///             resolvers.push(tail.serialize(serializer)?);
///         }
///         // Now we align our serializer for our archived type and resolve it.
///         // We can't align for unsized types so we treat the trailing slice
///         // like an array of 0 length for now.
///         let result = serializer
///             .align_for::<Block<Archived<H>, [Archived<T>; 0]>>()?;
///         unsafe {
///             serializer.resolve_aligned(&self.head, head_resolver)?;
///         }
///         serializer.align_for::<Archived<T>>()?;
///         for (item, resolver) in self.tail.iter().zip(resolvers.drain(..)) {
///             unsafe {
///                 serializer.resolve_aligned(item, resolver)?;
///             }
///         }
///         Ok(result)
///     }
/// }
///
/// let value = Box::new(Block {
///     head: "Numbers 1-4".to_string(),
///     tail: [1, 2, 3, 4],
/// });
///
/// // We have a Box<Block<String, [i32; 4]>> but we want to it to be a
/// // Box<Block<String, [i32]>>, so we need manually "unsize" the pointer.
/// let ptr = Box::into_raw(value);
/// let unsized_ptr = ptr_meta::from_raw_parts_mut::<Block<String, [i32]>>(
///     ptr.cast::<()>(),
///     4,
/// );
/// let unsized_value = unsafe { Box::from_raw(unsized_ptr) };
///
/// let bytes = to_bytes::<Error>(&unsized_value).unwrap();
///
/// let archived = unsafe {
///     access_unchecked::<Archived<Box<Block<String, [i32]>>>>(&bytes)
/// };
/// assert_eq!(archived.head, "Numbers 1-4");
/// assert_eq!(archived.tail.len(), 4);
/// assert_eq!(archived.tail, [1, 2, 3, 4]);
/// ```
pub trait ArchiveUnsized: Pointee {
    /// The archived counterpart of this type. Unlike `Archive`, it may be
    /// unsized.
    ///
    /// This type must implement [`ArchivePointee`], a trait that helps make
    /// valid pointers using archived pointer metadata.
    type Archived: ArchivePointee + Portable + ?Sized;

    /// Creates the archived version of the metadata for this value.
    fn archived_metadata(&self) -> ArchivedMetadata<Self>;
}

/// An archived type with associated metadata for its relative pointer.
///
/// This is mostly used in the context of smart pointers and unsized types, and
/// is implemented for all sized types by default.
pub trait ArchivePointee: Pointee {
    /// The archived version of the pointer metadata for this type.
    type ArchivedMetadata: Copy
        + Send
        + Sync
        + Ord
        + Hash
        + Unpin
        + Portable
        + Initialized;

    /// Converts some archived metadata to the pointer metadata for itself.
    fn pointer_metadata(
        archived: &Self::ArchivedMetadata,
    ) -> <Self as Pointee>::Metadata;
}

/// A counterpart of [`Serialize`] that's suitable for unsized types.
///
/// See [`ArchiveUnsized`] for examples of implementing `SerializeUnsized`.
pub trait SerializeUnsized<S: Fallible + ?Sized>: ArchiveUnsized {
    /// Writes the object and returns the position of the archived type.
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error>;

    /// Archives a reference to the given object and returns the position it was
    /// archived at.
    #[inline]
    fn serialize_and_resolve_rel_ptr(
        &self,
        serializer: &mut S,
    ) -> Result<usize, S::Error>
    where
        S: Writer,
    {
        let to = self.serialize_unsized(serializer)?;
        serializer.align_for::<RelPtr<Self::Archived>>()?;
        unsafe { serializer.resolve_unsized_aligned(self, to) }
    }
}

/// A counterpart of [`Deserialize`] that's suitable for unsized types.
pub trait DeserializeUnsized<T: Pointee + ?Sized, D: Fallible + ?Sized>:
    ArchivePointee
{
    /// Deserializes a reference to the given value.
    ///
    /// # Safety
    ///
    /// `out` must be non-null, properly-aligned, and valid for writes. It must
    /// be allocated according to the layout of the deserialized metadata.
    unsafe fn deserialize_unsized(
        &self,
        deserializer: &mut D,
        out: *mut T,
    ) -> Result<(), D::Error>;

    /// Deserializes the metadata for the given type.
    fn deserialize_metadata(
        &self,
        deserializer: &mut D,
    ) -> Result<T::Metadata, D::Error>;
}
