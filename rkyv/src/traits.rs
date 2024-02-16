//! The core traits provided by rkyv.

use core::{alloc::Layout, hash::Hash};

use crate::{
    ptr_meta::Pointee,
    rancor::Fallible,
    ser::{Writer, WriterExt as _},
    ArchivedMetadata, RelPtr,
};

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
/// use rkyv::{Archive, Deserialize, Serialize};
///
/// #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
/// // This will generate a PartialEq impl between our unarchived and archived types
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
/// let bytes = rkyv::to_bytes::<_, 256>(&value).unwrap();
///
/// // Or you can customize your serialization for better performance
/// // and compatibility with #![no_std] environments
/// use rkyv::ser::{Serializer, serializers::AllocSerializer};
///
/// let mut serializer = AllocSerializer::<0>::default();
/// serializer.serialize_value(&value).unwrap();
/// let bytes = serializer.into_serializer().into_inner();
///
/// // You can use the safe API with the `bytecheck` feature enabled,
/// // or you can use the unsafe API (shown here) for maximum performance
/// let archived = unsafe { rkyv::archived_root::<Test>(&bytes[..]) };
/// assert_eq!(archived, &value);
///
/// // And you can always deserialize back to the original type
/// let deserialized: Test = archived.deserialize(&mut rkyv::Infallible).unwrap();
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
/// use rkyv::{
///     archived_root,
///     ser::{Serializer, serializers::AlignedSerializer},
///     out_field,
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
///     // This is the resolver we can create our Archived version from.
///     type Resolver = OwnedStrResolver;
///
///     // The resolve function consumes the resolver and produces the archived
///     // value at the given position.
///     unsafe fn resolve(
///         &self,
///         pos: usize,
///         resolver: Self::Resolver,
///         out: *mut Self::Archived,
///     ) {
///         // We have to be careful to add the offset of the ptr field,
///         // otherwise we'll be using the position of the ArchivedOwnedStr
///         // instead of the position of the relative pointer.
///         let (fp, fo) = out_field!(out.ptr);
///         self.inner.resolve_unsized(
///             pos + fp,
///             resolver.pos,
///             resolver.metadata_resolver,
///             fo,
///         );
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
    /// The archived representation of this type.
    ///
    /// In this form, the data can be used with zero-copy deserialization.
    type Archived;

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
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing this object
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    );
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
/// use core::{mem::transmute, ops::{Deref, DerefMut}};
/// use ptr_meta::Pointee;
/// use rkyv::{
///     archived_unsized_value,
///     ser::{serializers::AlignedSerializer, Serializer},
///     AlignedVec,
///     Archive,
///     Archived,
///     ArchivedMetadata,
///     ArchivePointee,
///     ArchiveUnsized,
///     RelPtr,
///     Serialize,
///     SerializeUnsized,
///     primitive::ArchivedUsize,
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
///         archived.len.to_native() as usize
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
///     unsafe fn resolve_metadata(
///         &self,
///         _: usize,
///         _: Self::MetadataResolver,
///         out: *mut ArchivedMetadata<Self>,
///     ) {
///         unsafe {
///             out.write(BlockSliceMetadata {
///                 len: ArchivedUsize::from_native(self.tail.len() as _),
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
    /// The archived counterpart of this type. Unlike `Archive`, it may be
    /// unsized.
    ///
    /// This type must implement [`ArchivePointee`], a trait that helps make
    /// valid pointers using archived pointer metadata.
    type Archived: ArchivePointee + ?Sized;

    /// Creates the archived version of the metadata for this value.
    fn archived_metadata(&self) -> ArchivedMetadata<Self>;
}

/// An archived type with associated metadata for its relative pointer.
///
/// This is mostly used in the context of smart pointers and unsized types, and
/// is implemented for all sized types by default.
pub trait ArchivePointee: Pointee {
    /// The archived version of the pointer metadata for this type.
    type ArchivedMetadata: Copy + Send + Sync + Ord + Hash + Unpin;

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
    /// `out` must point to memory with the layout returned by
    /// `deserialized_layout`.
    unsafe fn deserialize_unsized(
        &self,
        deserializer: &mut D,
        alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), D::Error>;

    /// Deserializes the metadata for the given type.
    fn deserialize_metadata(
        &self,
        deserializer: &mut D,
    ) -> Result<T::Metadata, D::Error>;
}
