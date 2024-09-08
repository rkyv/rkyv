//! Trait object serialization for rkyv.
//!
//! With `rkyv_dyn`, trait objects can be serialized with rkyv then the methods
//! can be called without deserializing. All it takes is some macro magic.
//!
//! See [`SerializeDyn`] for an example of how to use rkyv_dyn.
//!
//! ## Features
//!
//! - `bytecheck`: Enables validation support through `bytecheck`.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]

mod lazy_static;
// TODO: re-enable
// #[cfg(feature = "bytecheck")]
// mod bytecheck;

use core::{hash, marker::PhantomData};

pub use lazy_static::LazyStatic;
use ptr_meta::{DynMetadata, Pointee};
use rancor::Fallible;
use rkyv::{
    de::Pooling,
    place::Initialized,
    primitive::FixedUsize,
    ser::{Allocator, Sharing, Writer},
    Archived, Portable, Serialize, SerializeUnsized,
};
pub use rkyv_dyn_derive::archive_dyn;

/// The type of trait impl IDs.
pub type ImplId = FixedUsize;

/// An object-safe version of `Serializer`.
///
/// Instead of an associated error type, `DynSerializer` returns the `E` type.
/// If you have a serializer that already implements `Serializer`, then it will
/// automatically implement `DynSerializer`.
pub trait DynSerializer<E>: Writer<E> + Allocator<E> + Sharing<E> {}

impl<E> Fallible for dyn DynSerializer<E> + '_ {
    type Error = E;
}

impl<S: Writer<E> + Allocator<E> + Sharing<E>, E> DynSerializer<E> for S {}

/// TODO
pub trait AsDynSerializer<E> {
    /// TODO
    fn as_dyn_serializer(&mut self) -> &mut dyn DynSerializer<E>;
}

impl<S: DynSerializer<E>, E> AsDynSerializer<E> for S {
    fn as_dyn_serializer(&mut self) -> &mut dyn DynSerializer<E> {
        self as &mut dyn DynSerializer<E>
    }
}

impl<E> AsDynSerializer<E> for dyn DynSerializer<E> {
    fn as_dyn_serializer(&mut self) -> &mut dyn DynSerializer<E> {
        self
    }
}

/// A trait object that can be archived.
///
/// To add archive support for a trait object:
///
/// 1. Add [`archive_dyn`](macro@archive_dyn) on your trait to make a
///    serializable version of it. By default, it will be named "Serialize" +
///    your trait name. To rename the trait, pass the argument `serialize =
///    "..."` as a parameter.
/// 2. Implement your trait for your type and add the attribute `#[archive_dyn]`
///    to it. Make sure to implement your trait for your archived type as well.
///    This invocation must have the same attributes as the trait invocation.
/// 3. If deserialization support is desired, add `deserialize` or `deserialize
///    = "..."` as parameters and implement `Deserialize` for the type. By
///    default, the deserialize trait will be named "Deserialize" + your trait
///    name. Passing a trait name will use that name instead.
///
/// Then you're ready to serialize boxed trait objects!
///
/// Even though your deserialized values are boxed as serialize trait objects,
/// your archived values are boxed as regular trait objects. This is because
/// your deserialized values have to implement `SerializeDyn` but your archived
/// values do not.
///
/// # Examples
///
/// See [`archive_dyn`](macro@archive_dyn) for customization options.
///
/// ```
/// use rkyv::{
///     archived_value,
///     ser::{serializers::AllocSerializer, Serializer},
///     AlignedVec, Archive, Archived, Deserialize, Infallible, Serialize,
/// };
/// use rkyv_dyn::archive_dyn;
///
/// #[archive_dyn(deserialize)]
/// trait ExampleTrait {
///     fn value(&self) -> String;
/// }
///
/// #[derive(Archive, Serialize, Deserialize)]
/// struct StringStruct(String);
///
/// #[archive_dyn(deserialize)]
/// impl ExampleTrait for StringStruct {
///     fn value(&self) -> String {
///         self.0.clone()
///     }
/// }
///
/// impl ExampleTrait for ArchivedStringStruct {
///     fn value(&self) -> String {
///         self.0.as_str().to_string()
///     }
/// }
///
/// #[derive(Archive, Serialize, Deserialize)]
/// struct IntStruct(i32);
///
/// #[archive_dyn(deserialize)]
/// impl ExampleTrait for IntStruct {
///     fn value(&self) -> String {
///         format!("{}", self.0)
///     }
/// }
///
/// impl ExampleTrait for ArchivedIntStruct {
///     fn value(&self) -> String {
///         format!("{}", self.0)
///     }
/// }
///
/// let boxed_int = Box::new(IntStruct(42)) as Box<dyn SerializeExampleTrait>;
/// let boxed_string = Box::new(StringStruct("hello world".to_string()))
///     as Box<dyn SerializeExampleTrait>;
/// let mut serializer = AllocSerializer::<256>::default();
/// let int_pos = serializer
///     .serialize_value(&boxed_int)
///     .expect("failed to archive boxed int");
/// let str_pos = serializer
///     .serialize_value(&boxed_string)
///     .expect("failed to archive boxed string");
/// let buf = serializer.into_serializer().into_inner();
/// let archived_int = unsafe {
///     archived_value::<Box<dyn SerializeExampleTrait>>(buf.as_ref(), int_pos)
/// };
/// let archived_string = unsafe {
///     archived_value::<Box<dyn SerializeExampleTrait>>(buf.as_ref(), str_pos)
/// };
/// assert_eq!(archived_int.value(), "42");
/// assert_eq!(archived_string.value(), "hello world");
///
/// let deserialized_int: Box<dyn SerializeExampleTrait> =
///     archived_int.deserialize(&mut Infallible).unwrap();
/// let deserialized_string: Box<dyn SerializeExampleTrait> =
///     archived_string.deserialize(&mut Infallible).unwrap();
/// assert_eq!(deserialized_int.value(), "42");
/// assert_eq!(deserialized_string.value(), "hello world");
/// ```
// TODO: This is just `for<'a> SerializeUnsized<dyn DynSerializer<E>> + 'a`
pub trait SerializeDyn<E> {
    /// Serializes this value and returns the position it is located at.
    fn serialize_dyn(
        &self,
        serializer: &mut dyn DynSerializer<E>,
    ) -> Result<usize, E>;
}

impl<T: for<'a> Serialize<dyn DynSerializer<E> + 'a>, E> SerializeDyn<E> for T {
    fn serialize_dyn(
        &self,
        serializer: &mut dyn DynSerializer<E>,
    ) -> Result<usize, E> {
        self.serialize_unsized(serializer)
    }
}

/// An object-safe version of `Deserializer`.
pub trait DynDeserializer<E>: Pooling<E> {}

impl<E> Fallible for dyn DynDeserializer<E> + '_ {
    type Error = E;
}

impl<D, E> DynDeserializer<E> for D where D: Pooling<E> {}

/// TODO
pub trait AsDynDeserializer<E> {
    /// TODO
    fn as_dyn_deserializer(&mut self) -> &mut dyn DynDeserializer<E>;
}

impl<S: DynDeserializer<E>, E> AsDynDeserializer<E> for S {
    fn as_dyn_deserializer(&mut self) -> &mut dyn DynDeserializer<E> {
        self as &mut dyn DynDeserializer<E>
    }
}

impl<E> AsDynDeserializer<E> for dyn DynDeserializer<E> {
    fn as_dyn_deserializer(&mut self) -> &mut dyn DynDeserializer<E> {
        self
    }
}

/// A trait object that can be deserialized.
///
/// See [`SerializeDyn`] for more information.
pub trait DeserializeDyn<T: Pointee + ?Sized, E> {
    /// Deserializes this value into the given out pointer.
    fn deserialize_dyn(
        &self,
        deserializer: &mut dyn DynDeserializer<E>,
        out: *mut T,
    ) -> Result<(), E>;

    /// Returns the pointer metadata for the deserialized form of this type.
    fn deserialized_pointer_metadata(&self) -> DynMetadata<T>;
}

/// The archived version of `DynMetadata`.
#[derive(Portable)]
#[cfg_attr(
    feature = "bytecheck",
    derive(::bytecheck::CheckBytes),
    bytecheck(verify)
)]
#[repr(transparent)]
pub struct ArchivedDynMetadata<T: ?Sized> {
    impl_id: Archived<ImplId>,
    phantom: PhantomData<T>,
}

// SAFETY: `ArchivedDynMetadata<T>` is a transparent wrapper around an archived
// `ImplId`, so if that archived type is initialized then so is
// `ArchivedDynMetadata<T>`.
unsafe impl<T: ?Sized> Initialized for ArchivedDynMetadata<T> where
    Archived<ImplId>: Initialized
{
}

impl<T: ?Sized> Copy for ArchivedDynMetadata<T> {}
unsafe impl<T: ?Sized> Send for ArchivedDynMetadata<T> {}
unsafe impl<T: ?Sized> Sync for ArchivedDynMetadata<T> {}
impl<T: ?Sized> Unpin for ArchivedDynMetadata<T> {}

impl<T: ?Sized> ArchivedDynMetadata<T> {
    /// Creates a new `ArchivedDynMetadata` for the given type.
    ///
    /// # Safety
    ///
    /// `out` must point to a valid location for an `ArchivedDynMetadata<T>`.
    pub fn new(impl_id: ImplId) -> Self {
        Self {
            impl_id: Archived::<ImplId>::from_native(impl_id),
            phantom: PhantomData,
        }
    }

    /// Returns the impl ID of the associated with this `ArchivedDynMetadata`.
    pub fn impl_id(&self) -> ImplId {
        self.impl_id.to_native()
    }

    /// Returns the pointer metadata for the trait object this metadata refers
    /// to.
    pub fn lookup_metadata(&self) -> DynMetadata<T> {
        unsafe {
            TRAIT_IMPLS
                .get()
                .expect("TRAIT_IMPLS was not initialized for rkyv_dyn")
                [self.impl_id() as usize]
                .downcast_metadata()
        }
    }
}

impl<T: ?Sized> Clone for ArchivedDynMetadata<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> hash::Hash for ArchivedDynMetadata<T> {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.impl_id.hash(state);
    }
}

impl<T: ?Sized> PartialEq for ArchivedDynMetadata<T> {
    fn eq(&self, other: &Self) -> bool {
        self.impl_id.eq(&other.impl_id)
    }
}

impl<T: ?Sized> Eq for ArchivedDynMetadata<T> {}

impl<T: ?Sized> PartialOrd for ArchivedDynMetadata<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: ?Sized> Ord for ArchivedDynMetadata<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.impl_id.cmp(&other.impl_id)
    }
}

/// The trait object metadata for a trait implementation.
#[derive(Clone, Copy, Debug)]
pub struct TraitImpl {
    // The type of this `DynMetadata` is erased. Whatever uses it will
    // transmute it to the correct `DynMetadata<T>`.
    metadata: DynMetadata<()>,
}

impl TraitImpl {
    /// Creates a new trait impl from a trait object pointer.
    ///
    /// # Safety
    ///
    /// `pointer` must have valid metadata.
    pub unsafe fn from_pointer<
        T: Pointee<Metadata = DynMetadata<T>> + ?Sized,
    >(
        pointer: *const T,
    ) -> Self {
        Self::from_metadata(ptr_meta::metadata(pointer))
    }

    /// Creates a new trait impl from its trait object metadata.
    pub fn from_metadata<T: ?Sized>(metadata: DynMetadata<T>) -> Self {
        Self {
            // SAFETY: All `DynMetadata<T>` have the same layout and validity.
            // They all contain a single erased `&'static VTable` reference and
            // a `PhantomData<T>`.
            metadata: unsafe { core::mem::transmute(metadata) },
        }
    }

    /// Returns the trait object metadata of this trait implementation downcast
    /// to the given type.
    ///
    /// # Safety
    ///
    /// `T` must be the `dyn Trait` that this `TraitImpl` corresponds to.
    pub unsafe fn downcast_metadata<T: ?Sized>(&self) -> DynMetadata<T> {
        unsafe { core::mem::transmute(self.metadata) }
    }
}

/// Creates a new [`TraitImpl`] from the given type and dyn trait.
///
/// See [`register_trait_impls`] for a macro that registers these trait impls
/// globally.
///
/// # Example
/// ```
/// struct MyType;
///
/// trait MyTrait {}
///
/// impl MyTrait for MyType {}
///
/// let trait_impl = trait_impl!(MyType as dyn MyTrait);
/// ```
#[macro_export]
macro_rules! trait_impl {
    ($type:ty as $trait:ty) => {
        // SAFETY: The given pointer is guaranteed to have valid metadata
        // because we just made them.
        unsafe {
            $crate::TraitImpl::from_pointer(
                ::core::ptr::null::<$type>() as *const $trait
            )
        }
    };
}

/// All registered trait impls for `rkyv_dyn`.
///
/// This can be initialized with
pub static TRAIT_IMPLS: LazyStatic<&'static [TraitImpl]> = LazyStatic::new();

/// Globally registers the given trait impls. This macro performs three basic
/// functions:
///
/// 1. Generating `impl RegisteredImpl<$trait> for $type` definitions with valid
///    impl IDs.
/// 2. Creating and initializing a static array of [`TraitImpl`]s, one for each
///    trait impl argument.
/// 3. Initializing [`TRAIT_IMPLS`] with a reference to the array of
///    [`TraitImpl`]s.
#[macro_export]
macro_rules! register_trait_impls {
    ($($type:ty as $trait:ty $(= $id:expr)?),* $(,)?) => {
        let _: () = {
            $crate::register_trait_impls!(
                @register $($type as $trait $(= $id)?,)*
            );
            const TRAIT_IMPL_COUNT: usize = 0
                $(+ { let _ = ::core::marker::PhantomData::<$type>; 1 })*;
            static TRAIT_IMPLS: $crate::LazyStatic<[
                $crate::TraitImpl;
                TRAIT_IMPL_COUNT
            ]> = $crate::LazyStatic::new();
            let trait_impls = TRAIT_IMPLS.init([
                $(
                    $crate::trait_impl!($type as $trait),
                )*
            ]).unwrap();
            $crate::TRAIT_IMPLS.init(trait_impls).unwrap();
        };
    };
    (
        @register
        $first_type:ty as $first_trait:ty $(= $first_id:expr)?,
        $($rest_type:ty as $rest_trait:ty $(= $rest_id:expr)?,)*
    ) => {
        struct ImplIds;

        trait Registered<const ID: $crate::ImplId> {}

        unsafe impl $crate::RegisteredImpl<$first_trait> for $first_type {
            const IMPL_ID: $crate::ImplId =
                $crate::register_trait_impls!(@choose_id 0, $($first_id)?);
        }
        impl Registered<
            { <$first_type as $crate::RegisteredImpl<$first_trait>>::IMPL_ID }
        > for ImplIds {}

        $crate::register_trait_impls!(
            @register_rest $first_type as $first_trait,
            $($rest_type:ty as $rest_trait:ty $(= $rest_id:expr)?,)*
        );
    };
    (@register_rest $prev_type:ty as $prev_trait:ty,) => {};
    (
        @register_rest
        $prev_type:ty as $prev_trait:ty,
        $type:ty as $trait:ty $(= $id:expr)?,
        $($rest_type:ty as $rest_trait:ty $(= $rest_id:expr)?,)*
    ) => {
        unsafe impl $crate::RegisteredImpl<$trait> for $type {
            const IMPL_ID: $crate::ImplId = $crate::register_trait_impls!(
                @choose_id
                <
                    $prev_type as $crate::RegisteredImpl<$prev_trait>
                >::IMPL_ID + 1,
                $($id)?
            );
        }
        impl Registered<{
            <$type as $crate::RegisteredImpl<$trait>>::IMPL_ID
        }> for ImplIds {}

        $crate::register_trait_impls!(
            @register_rest $type as $trait,
            $($rest_type as $rest_trait $(= $rest_id)?,)*
        );
    };
    (@choose_id $default:expr, $explicit:expr) => { $explicit };
    (@choose_id $default:expr,) => { $default };
}

/// A trait impl that has a globally-unique ID.
///
/// # Safety
///
/// `IMPL_ID` must be globally unique.
pub unsafe trait RegisteredImpl<T: ?Sized> {
    /// The ID of this trait impl.
    const IMPL_ID: ImplId;
}
