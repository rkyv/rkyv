//! Trait object serialization for rkyv.
//!
//! With `rkyv_dyn`, trait objects can be serialized with rkyv then the methods can be called
//! without deserializing. All it takes is some macro magic.
//!
//! See [`SerializeDyn`] for an example of how to use rkyv_dyn.
//!
//! ## Features
//!
//! - `nightly`: Enables some nightly features, such as [`likely`](std::intrinsics::likely).
//! - `strict`: Guarantees that types will have the same representations across platforms and
//!   compilations. This is already the case in practice, but this feature provides a guarantee.
//! - `validation`: Enables validation support through `bytecheck`.
//! - `vtable_cache`: Enables local vtable caching to speed up lookups after the first. This
//!   requires mutating the archive, which is not possible for all use cases.

#![deny(rustdoc::broken_intra_doc_links)]
#![deny(missing_docs)]
#![deny(rustdoc::missing_crate_level_docs)]
#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "vtable_cache")]
use core::sync::atomic::{AtomicU64, Ordering};
use core::{
    alloc::Layout,
    any::Any,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ptr,
};
use ptr_meta::{DynMetadata, Pointee};
#[cfg(feature = "vtable_cache")]
use rkyv::with::{Atomic, With};
use rkyv::{
    primitive::ArchivedU64,
    ser::{ScratchSpace, Serializer},
    Fallible, Serialize,
};
pub use rkyv_dyn_derive::archive_dyn;
use rkyv_typename::TypeName;
use std::collections::{hash_map::DefaultHasher, HashMap};
#[cfg(feature = "validation")]
pub use validation::{CheckDynError, DynContext};

#[doc(hidden)]
pub use inventory;

#[cfg(all(feature = "vtable_cache", feature = "nightly"))]
use core::intrinsics::likely;
#[cfg(all(feature = "vtable_cache", not(feature = "nightly")))]
#[inline]
fn likely(b: bool) -> bool {
    b
}

/// An error that can occur while serializing and deserializing trait objects.
pub type DynError = Box<dyn Any>;

/// An object-safe version of `Serializer`.
///
/// Instead of an associated error type, `DynSerializer` returns the [`DynError`] type. If you have
/// a serializer that already implements `Serializer`, then it will automatically implement
/// `DynSerializer`.
pub trait DynSerializer {
    /// Returns the current position of the serializer.
    fn pos_dyn(&self) -> usize;

    /// Attempts to write the given bytes to the serializer.
    fn write_dyn(&mut self, bytes: &[u8]) -> Result<(), DynError>;

    /// Allocates scratch space of the requested size.
    ///
    /// # Safety
    ///
    /// `layout` must have non-zero size.
    unsafe fn push_scratch_dyn(
        &mut self,
        layout: Layout,
    ) -> Result<ptr::NonNull<[u8]>, DynError>;

    /// Deallocates previously allocated scratch space.
    ///
    /// # Safety
    ///
    /// - `ptr` must be the scratch memory last allocated with `push_scratch`.
    /// - `layout` must be the same layout that was used to allocate that block of memory.
    unsafe fn pop_scratch_dyn(
        &mut self,
        ptr: ptr::NonNull<u8>,
        layout: Layout,
    ) -> Result<(), DynError>;
}

impl<'a> Fallible for dyn DynSerializer + 'a {
    type Error = DynError;
}

impl<'a> Serializer for dyn DynSerializer + 'a {
    fn pos(&self) -> usize {
        self.pos_dyn()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        self.write_dyn(bytes)
    }
}

impl<'a> ScratchSpace for dyn DynSerializer + 'a {
    unsafe fn push_scratch(
        &mut self,
        layout: Layout,
    ) -> Result<ptr::NonNull<[u8]>, Self::Error> {
        self.push_scratch_dyn(layout)
    }

    unsafe fn pop_scratch(
        &mut self,
        ptr: ptr::NonNull<u8>,
        layout: Layout,
    ) -> Result<(), Self::Error> {
        self.pop_scratch_dyn(ptr, layout)
    }
}

impl<S: ScratchSpace + Serializer + ?Sized> DynSerializer for &mut S {
    fn pos_dyn(&self) -> usize {
        self.pos()
    }

    fn write_dyn(&mut self, bytes: &[u8]) -> Result<(), DynError> {
        self.write(bytes).map_err(|e| Box::new(e) as DynError)
    }

    unsafe fn push_scratch_dyn(
        &mut self,
        layout: Layout,
    ) -> Result<ptr::NonNull<[u8]>, DynError> {
        self.push_scratch(layout)
            .map_err(|e| Box::new(e) as DynError)
    }

    unsafe fn pop_scratch_dyn(
        &mut self,
        ptr: ptr::NonNull<u8>,
        layout: Layout,
    ) -> Result<(), DynError> {
        self.pop_scratch(ptr, layout)
            .map_err(|e| Box::new(e) as DynError)
    }
}

fn hash_type<T: TypeName + ?Sized>() -> u64 {
    let mut hasher = DefaultHasher::new();
    T::build_type_name(|piece| piece.hash(&mut hasher));
    hasher.finish()
}

/// A trait object that can be archived.
///
/// To add archive support for a trait object:
///
/// 1. Add [`archive_dyn`](macro@archive_dyn) on your trait to make a serializable version of it. By
///    default, it will be named "Serialize" + your trait name. To rename the trait, pass the
///    argument `serialize = "..."` as a parameter.
/// 2. Implement `Archive` and `Serialize` for the type you want to make trait objects of and
///    `TypeName` for the archived versions of them.
/// 3. Implement your trait for your type and add the attribute `#[archive_dyn]` to it. Make sure to
///    implement your trait for your archived type as well. This invocation must have the same
///    attributes as the trait invocation.
/// 4. If deserialization support is desired, add `deserialize` or `deserialize = "..."` as
///    parameters and implement `Deserialize` for the type. By default, the deserialize trait will
///    be named "Deserialize" + your trait name. Passing a trait name will use that name instead.
///
/// Then you're ready to serialize boxed trait objects!
///
/// Even though your deserialized values are boxed as serialize trait objects, your archived values
/// are boxed as regular trait objects. This is because your deserialized values have to implement
/// `SerializeDyn` but your archived values do not.
///
/// # Examples
///
/// See [`archive_dyn`](macro@archive_dyn) for customization options.
///
/// ```
/// use rkyv::{
///     archived_value,
///     ser::{
///         serializers::AllocSerializer,
///         Serializer,
///     },
///     AlignedVec,
///     Archive,
///     Archived,
///     Deserialize,
///     Infallible,
///     Serialize,
/// };
/// use rkyv_dyn::archive_dyn;
/// use rkyv_typename::TypeName;
///
/// #[archive_dyn(deserialize)]
/// trait ExampleTrait {
///     fn value(&self) -> String;
/// }
///
/// #[derive(Archive, Serialize, Deserialize)]
/// #[archive_attr(derive(TypeName))]
/// struct StringStruct(String);
///
/// #[archive_dyn(deserialize)]
/// impl ExampleTrait for StringStruct {
///     fn value(&self) -> String {
///         self.0.clone()
///     }
/// }
///
/// impl ExampleTrait for Archived<StringStruct> {
///     fn value(&self) -> String {
///         self.0.as_str().to_string()
///     }
/// }
///
/// #[derive(Archive, Serialize, Deserialize)]
/// #[archive_attr(derive(TypeName))]
/// struct IntStruct(i32);
///
/// #[archive_dyn(deserialize)]
/// impl ExampleTrait for IntStruct {
///     fn value(&self) -> String {
///         format!("{}", self.0)
///     }
/// }
///
/// impl ExampleTrait for Archived<IntStruct> {
///     fn value(&self) -> String {
///         format!("{}", self.0)
///     }
/// }
///
/// let boxed_int = Box::new(IntStruct(42)) as Box<dyn SerializeExampleTrait>;
/// let boxed_string = Box::new(StringStruct("hello world".to_string()))
///     as Box<dyn SerializeExampleTrait>;
/// let mut serializer = AllocSerializer::<256>::default();
/// let int_pos = serializer.serialize_value(&boxed_int)
///     .expect("failed to archive boxed int");
/// let str_pos = serializer.serialize_value(&boxed_string)
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
/// let deserialized_int: Box<dyn SerializeExampleTrait> = archived_int
///     .deserialize(&mut Infallible).unwrap();
/// let deserialized_string: Box<dyn SerializeExampleTrait> = archived_string
///     .deserialize(&mut Infallible).unwrap();
/// assert_eq!(deserialized_int.value(), "42");
/// assert_eq!(deserialized_string.value(), "hello world");
/// ```
pub trait SerializeDyn {
    /// Writes the value to the serializer and returns the position it was written to.
    fn serialize_dyn(
        &self,
        serializer: &mut dyn DynSerializer,
    ) -> Result<usize, DynError>;

    /// Returns the type ID of the archived version of this type.
    fn archived_type_id(&self) -> u64;
}

impl<T: for<'a> Serialize<dyn DynSerializer + 'a>> SerializeDyn for T
where
    T::Archived: TypeName,
{
    fn serialize_dyn(
        &self,
        serializer: &mut dyn DynSerializer,
    ) -> Result<usize, DynError> {
        serializer.serialize_value(self)
    }

    fn archived_type_id(&self) -> u64 {
        hash_type::<T::Archived>()
    }
}

/// An object-safe version of `Deserializer`.
pub trait DynDeserializer {}

impl<'a> Fallible for dyn DynDeserializer + 'a {
    type Error = DynError;
}

impl<D: Fallible + ?Sized> DynDeserializer for &mut D {}

/// A trait object that can be deserialized.
///
/// See [`SerializeDyn`] for more information.
pub trait DeserializeDyn<T: Pointee + ?Sized> {
    /// Deserializes the given value as a trait object.
    ///
    /// # Safety
    ///
    /// The memory returned must be properly deallocated.
    unsafe fn deserialize_dyn(
        &self,
        deserializer: &mut dyn DynDeserializer,
        alloc: &mut dyn FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), DynError>;

    /// Returns the metadata for the deserialized version of this value.
    fn deserialize_dyn_metadata(
        &self,
        deserializer: &mut dyn DynDeserializer,
    ) -> Result<T::Metadata, DynError>;
}

/// The archived version of `DynMetadata`.
#[cfg_attr(feature = "strict", repr(C))]
pub struct ArchivedDynMetadata<T: ?Sized> {
    type_id: ArchivedU64,
    #[cfg(feature = "vtable_cache")]
    cached_vtable: ArchivedAtomicU64,
    #[cfg(not(feature = "vtable_cache"))]
    #[allow(dead_code)]
    cached_vtable: ArchivedU64,
    phantom: PhantomData<T>,
}

impl<T: TypeName + ?Sized> ArchivedDynMetadata<T> {
    /// Creates a new `ArchivedDynMetadata` for the given type.
    ///
    /// # Safety
    ///
    /// `out` must point to a valid location for an `ArchivedDynMetadata<T>`.
    pub unsafe fn emplace(type_id: u64, out: *mut Self) {
        ptr::addr_of_mut!((*out).type_id)
            .write(ArchivedU64::from_native(type_id));
        #[cfg(feature = "vtable_cache")]
        {
            let cached_vtable = ptr::addr_of_mut!((*out).cached_vtable);
            (*cached_vtable).store(0u64, Ordering::Relaxed);
        }
        #[cfg(not(feature = "vtable_cache"))]
        ptr::addr_of_mut!((*out).cached_vtable)
            .write(ArchivedU64::from_native(0u64));
    }

    fn lookup_vtable(&self) -> usize {
        IMPL_REGISTRY
            .get::<T>(self.type_id.to_native())
            .expect("attempted to get vtable for an unregistered impl")
            .vtable
    }

    /// Gets the vtable address for this trait object. With the `vtable_cache` feature, this will
    /// store the address locally on the first lookup.
    #[cfg(feature = "vtable_cache")]
    pub fn vtable(&self) -> usize {
        let cached_vtable = self.cached_vtable.load(Ordering::Relaxed);
        if likely(cached_vtable != 0) {
            return cached_vtable as usize;
        }
        let vtable = self.lookup_vtable();
        self.cached_vtable
            .store(vtable as usize as u64, Ordering::Relaxed);
        vtable
    }

    /// Gets the vtable address for this trait object. With the `vtable_cache` feature, this will
    /// store the address locally on the first lookup.
    #[cfg(not(feature = "vtable_cache"))]
    pub fn vtable(&self) -> usize {
        self.lookup_vtable()
    }

    /// Gets the `DynMetadata` associated with this `ArchivedDynMetadata`.
    pub fn pointer_metadata(&self) -> DynMetadata<T> {
        unsafe { core::mem::transmute(self.vtable()) }
    }
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplDebugInfo {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! debug_info {
    () => {
        rkyv_dyn::ImplDebugInfo {
            file: core::file!(),
            line: core::line!(),
            column: core::column!(),
        }
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplDebugInfo;

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! debug_info {
    () => {
        rkyv_dyn::ImplDebugInfo
    };
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ImplData {
    pub vtable: usize,
    pub debug_info: ImplDebugInfo,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
struct ImplId {
    trait_id: u64,
    type_id: u64,
}

impl ImplId {
    fn new<TY: TypeName, TR: TypeName + ?Sized>() -> Self {
        Self::from_type_id::<TR>(hash_type::<TY>())
    }

    fn from_type_id<TR: TypeName + ?Sized>(type_id: u64) -> Self {
        Self {
            trait_id: hash_type::<TR>(),
            // The last bit of the type ID is set to 1 to make sure we can differentiate between
            // cached and uncached vtables when the feature is turned on
            type_id: type_id | 1,
        }
    }
}

#[doc(hidden)]
pub struct ImplEntry {
    impl_id: ImplId,
    data: ImplData,
}

impl ImplEntry {
    #[doc(hidden)]
    pub fn new<TY: TypeName + RegisteredImpl<TR>, TR: TypeName + ?Sized>(
    ) -> Self {
        Self {
            impl_id: ImplId::new::<TY, TR>(),
            data: ImplData {
                vtable: <TY as RegisteredImpl<TR>>::vtable(),
                debug_info: <TY as RegisteredImpl<TR>>::debug_info(),
            },
        }
    }
}

inventory::collect!(ImplEntry);

struct ImplRegistry {
    id_to_data: HashMap<ImplId, ImplData>,
}

impl ImplRegistry {
    fn new() -> Self {
        Self {
            id_to_data: HashMap::new(),
        }
    }

    fn add_entry(&mut self, entry: &ImplEntry) {
        let old_value = self.id_to_data.insert(entry.impl_id, entry.data);

        #[cfg(debug_assertions)]
        if let Some(old_data) = old_value {
            eprintln!("impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
            eprintln!(
                "existing impl registered at {}:{}:{}",
                old_data.debug_info.file,
                old_data.debug_info.line,
                old_data.debug_info.column
            );
            eprintln!(
                "new impl registered at {}:{}:{}",
                entry.data.debug_info.file,
                entry.data.debug_info.line,
                entry.data.debug_info.column
            );
            panic!();
        }

        debug_assert!(old_value.is_none(), "impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    fn get<T: TypeName + ?Sized>(&self, type_id: u64) -> Option<&ImplData> {
        self.id_to_data.get(&ImplId::from_type_id::<T>(type_id))
    }
}

lazy_static::lazy_static! {
    static ref IMPL_REGISTRY: ImplRegistry = {
        let mut result = ImplRegistry::new();
        for entry in inventory::iter::<ImplEntry> {
            result.add_entry(entry);
        }
        result
    };
}

/// Guarantees that an impl has been registered for the type as the given trait object.
#[doc(hidden)]
pub unsafe trait RegisteredImpl<T: ?Sized> {
    fn vtable() -> usize;
    fn debug_info() -> ImplDebugInfo;
}

#[doc(hidden)]
#[cfg(not(feature = "validation"))]
#[macro_export]
macro_rules! register_validation {
    ($type:ty as $trait:ty) => {};
}

/// Registers a new impl with the trait object system.
///
/// This is called by `#[archive_dyn]` when attached to a trait implementation. You might need to
/// call this manually if you're using generic traits and types, since each specific instance needs
/// to be individually registered.
///
/// Call it like `register_impl!(MyType as dyn MyTrait)`.
#[macro_export]
macro_rules! register_impl {
    ($type:ty as $trait:ty) => {
        const _: () = {
            use rkyv_dyn::{
                debug_info, inventory, register_validation, ImplData,
                ImplDebugInfo, ImplEntry, RegisteredImpl,
            };

            unsafe impl RegisteredImpl<$trait> for $type {
                fn vtable() -> usize {
                    unsafe {
                        core::mem::transmute(ptr_meta::metadata(
                            core::ptr::null::<$type>() as *const $trait,
                        ))
                    }
                }

                fn debug_info() -> ImplDebugInfo {
                    debug_info!()
                }
            }

            inventory::submit! { ImplEntry::new::<$type, $trait>() }
            register_validation!($type as $trait);
        };
    };
}
