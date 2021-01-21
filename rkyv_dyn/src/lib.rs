//! Trait object serialization for rkyv.
//!
//! With `rkyv_dyn`, trait objects can be serialized with rkyv then the methods
//! can be called without deserializing. All it takes is some macro magic.
//!
//! See [`SerializeDyn`] for an example of how to use rkyv_dyn.
//!
//! ## Features
//!
//! - `nightly`: Enables some nightly features, such as
//!   [`likely`](std::intrinsics::likely).
//! - `strict`: Guarantees that types will have the same representations across
//!   platforms and compilations. This is already the case in practice, but this
//!   feature provides a guarantee.
//! - `validation`: Enables validation support through `bytecheck`.
//! - `vtable_cache`: Enables local vtable caching to speed up lookups after the
//!   first.

#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

#[cfg(feature = "validation")]
pub mod validation;

use core::{
    alloc,
    any::Any,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU64, Ordering},
};
use rkyv::{
    de::Deserializer,
    offset_of,
    ser::Serializer,
    Fallible,
    Serialize,
    RelPtr,
};
pub use rkyv_dyn_derive::archive_dyn;
use rkyv_typename::TypeName;
use std::collections::{hash_map::DefaultHasher, HashMap};

#[doc(hidden)]
pub use inventory;
#[cfg(feature = "validation")]
#[doc(hidden)]
pub use validation::ImplValidation;

#[cfg(all(feature = "vtable_cache", feature = "nightly"))]
use core::intrinsics::likely;
#[cfg(all(feature = "vtable_cache", not(feature = "nightly")))]
#[inline]
fn likely(b: bool) -> bool {
    b
}

/// A generic error that can be returned from a [`DynSerializer`].
pub type DynError = Box<dyn Any>;

/// An object-safe version of `Serializer`.
///
/// Instead of an associated error type, `DynSerializer` returns the
/// [`DynError`] type. If you have a serializer that already implements
/// `Serializer`, then it will automatically implement `DynSerializer`.
pub trait DynSerializer {
    /// Returns the current position of the serializer.
    fn pos_dyn(&self) -> usize;

    /// Attempts to write the given bytes to the serializer.
    fn write_dyn(&mut self, bytes: &[u8]) -> Result<(), DynError>;
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

impl<S: Serializer + ?Sized> DynSerializer for &mut S {
    fn pos_dyn(&self) -> usize {
        self.pos()
    }

    fn write_dyn(&mut self, bytes: &[u8]) -> Result<(), DynError> {
        match self.write(bytes) {
            Ok(()) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
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
/// 1. Add [`archive_dyn`](macro@archive_dyn) on your trait to make an
/// archive-compatible version of it. By default, it will be named "Archive" +
/// your trait name. To rename the trait, pass the argument `name = "..."` as
/// a parameter.
/// 2. Implement `Archive` for the type you want to make trait objects of and
/// `TypeName` for the archived versions of them.
/// 3. Implement your trait for your type and add the attribute `#[archive_dyn]`
/// to it. Make sure to implement your trait for your archived type as well.
/// This invocation must have the same attributes as the trait invocation.
/// 4. If deserialization support is desired, add `deserialize` or
/// `deserialize = "..."` as parameters. By default, the deserialize trait will
/// be named "Deserialize" + your trait name. Passing a trait name will use that
/// name instead.
///
/// Then you're ready to serialize boxed trait objects!
///
/// Even though your deserialized values are boxed as archive trait objects,
/// your archived values are boxed as regular trait objects. This is because
/// your deserialized values have to implement `ArchiveDyn` but your archived
/// values do not.
///
/// ## Examples
///
/// See [`archive_dyn`](macro@archive_dyn) for customization options.
///
/// ```
/// use rkyv::{
///     archived_value,
///     de::deserializers::AllocDeserializer,
///     ser::{
///         serializers::BufferSerializer,
///         Serializer,
///     },
///     Aligned,
///     Archive,
///     Archived,
///     Deserialize,
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
/// #[archive(derive(TypeName))]
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
/// #[archive(derive(TypeName))]
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
/// let boxed_string = Box::new(StringStruct("hello world".to_string())) as Box<dyn SerializeExampleTrait>;
/// let mut serializer = BufferSerializer::new(Aligned([0u8; 256]));
/// let int_pos = serializer.archive(&boxed_int)
///     .expect("failed to archive boxed int");
/// let string_pos = serializer.archive(&boxed_string)
///     .expect("failed to archive boxed string");
/// let buf = serializer.into_inner();
/// let archived_int = unsafe { archived_value::<Box<dyn SerializeExampleTrait>>(buf.as_ref(), int_pos) };
/// let archived_string = unsafe { archived_value::<Box<dyn SerializeExampleTrait>>(buf.as_ref(), string_pos) };
/// assert_eq!(archived_int.value(), "42");
/// assert_eq!(archived_string.value(), "hello world");
///
/// let deserialized_int: Box<dyn SerializeExampleTrait> = archived_int.deserialize(&mut AllocDeserializer).unwrap();
/// let deserialized_string: Box<dyn SerializeExampleTrait> = archived_string.deserialize(&mut AllocDeserializer).unwrap();
/// assert_eq!(deserialized_int.value(), "42");
/// assert_eq!(deserialized_string.value(), "hello world");
/// ```
pub trait SerializeDyn {
    /// Writes the value to the serializer and returns a resolver that can
    /// create an [`ArchivedDyn`] reference.
    fn serialize_dyn(&self, serializer: &mut dyn DynSerializer) -> Result<usize, DynError>;

    fn archived_type_id(&self) -> u64;
}

impl<T: for<'a> Serialize<dyn DynSerializer + 'a>> SerializeDyn for T
where
    T::Archived: TypeName,
{
    fn serialize_dyn(&self, serializer: &mut dyn DynSerializer) -> Result<usize, DynError> {
        Ok(serializer.archive(self)?)
    }

    fn archived_type_id(&self) -> u64 {
        hash_type::<T::Archived>()
    }
}

pub trait DynDeserializer {
    unsafe fn alloc_dyn(&mut self, layout: alloc::Layout) -> Result<*mut u8, DynError>;
}

impl<'a> Fallible for dyn DynDeserializer + 'a {
    type Error = DynError;
}

impl<'a> Deserializer for (dyn DynDeserializer + 'a) {
    unsafe fn alloc(&mut self, layout: alloc::Layout) -> Result<*mut u8, Self::Error> {
        self.alloc_dyn(layout)
    }
}

impl<D: Deserializer + ?Sized> DynDeserializer for &mut D {
    unsafe fn alloc_dyn(&mut self, layout: alloc::Layout) -> Result<*mut u8, DynError> {
        self.alloc(layout).map_err(|e| Box::new(e) as Box<dyn Any>)
    }
}

/// A trait object that can be deserialized.
///
/// See [`SerializeDyn`] for more information.
pub trait DeserializeDyn<T: ?Sized> {
    /// Deserializes the given value as a trait object.
    ///
    /// # Safety
    ///
    /// The return value must be allocated using the given allocator function.
    unsafe fn deserialize_dyn(&self, deserializer: &mut dyn DynDeserializer) -> Result<*mut T, DynError>;
}

/// A reference to an archived trait object.
///
/// This is essentially a pair of a data pointer and a type id. The
/// `vtable_cache` feature is recommended if your situation allows for it. With
/// `vtable_cache`, the vtable will only be looked up once and then stored
/// locally for subsequent lookups when the reference is dereferenced.
///
/// `ArchivedDyn` is the trait object extension of `RelPtr`.
#[cfg_attr(feature = "strict", repr(C))]
#[derive(Debug)]
pub struct ArchivedDyn<T: ?Sized> {
    ptr: RelPtr,
    type_id: AtomicU64,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> ArchivedDyn<T> {
    /// Creates a new `ArchivedDyn` from a type ID, position, and data position.
    pub fn new(type_id: u64, from: usize, to: usize) -> Self {
        Self {
            ptr: unsafe { RelPtr::new(from + offset_of!(Self, ptr), to) },
            // The last bit of the type ID is set to 1 to make sure we can
            // differentiate between cached and uncached vtables when the
            // feature is turned on
            type_id: AtomicU64::new(type_id | 1),
            _phantom: PhantomData,
        }
    }

    /// Gets the data pointer of the trait object.
    pub fn data_ptr(&self) -> *const () {
        self.ptr.as_ptr()
    }
}

impl<T: TypeName + ?Sized> ArchivedDyn<T> {
    /// Gets the vtable pointer for this trait object. With the `vtable_cache`
    /// feature, this will store the vtable locally on the first lookup.
    pub fn vtable(&self) -> *const () {
        let type_id = self.type_id.load(Ordering::Relaxed);

        #[cfg(feature = "vtable_cache")]
        if likely(type_id & 1 == 0) {
            return type_id as usize as *const ();
        }

        let ptr = IMPL_REGISTRY
            .data::<T>(type_id)
            .expect("attempted to get vtable for an unregistered impl")
            .vtable
            .0;

        #[cfg(feature = "vtable_cache")]
        self.type_id.store(ptr as usize as u64, Ordering::Relaxed);

        ptr
    }
}

impl<T: TypeName + ?Sized> Deref for ArchivedDyn<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = (self.data_ptr(), self.vtable());
        unsafe { &**(&ptr as *const (*const (), *const ()) as *const *const Self::Target) }
    }
}

impl<T: TypeName + ?Sized> DerefMut for ArchivedDyn<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = (self.data_ptr(), self.vtable());
        unsafe { &mut **(&ptr as *const (*const (), *const ()) as *const *mut Self::Target) }
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

#[cfg(not(feature = "validation"))]
#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct ImplValidation;

#[cfg(not(feature = "validation"))]
#[doc(hidden)]
#[macro_export]
macro_rules! validation {
    ($type:ty) => {
        ImplValidation
    };
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct ImplData {
    pub vtable: VTable,
    pub debug_info: ImplDebugInfo,
    pub validation: ImplValidation,
}

#[derive(Clone, Copy, Hash, Eq, PartialEq)]
struct ImplId {
    trait_id: u64,
    type_id: u64,
}

impl ImplId {
    fn new<TR: TypeName + ?Sized, TY: TypeName + ?Sized>() -> Self {
        Self::from_type_id::<TR>(hash_type::<TY>())
    }

    fn from_type_id<TR: TypeName + ?Sized>(type_id: u64) -> Self {
        Self {
            trait_id: hash_type::<TR>(),
            // The last bit of the type ID is set to 1 to make sure we can
            // differentiate between cached and uncached vtables when the
            // feature is turned on
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
    pub fn new<TR: TypeName + ?Sized, TY: TypeName + RegisteredImpl<TR> + ?Sized>() -> Self {
        Self {
            impl_id: ImplId::new::<TR, TY>(),
            data: <TY as RegisteredImpl<TR>>::data(),
        }
    }
}

inventory::collect!(ImplEntry);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct VTable(pub *const ());

unsafe impl Send for VTable {}
unsafe impl Sync for VTable {}

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
        #[cfg(feature = "vtable_cache")]
        debug_assert!(
            (entry.data.vtable.0 as usize) & 1 == 0,
            "vtable has a non-zero least significant bit which breaks vtable caching"
        );
        let old_value = self.id_to_data.insert(entry.impl_id, entry.data);

        #[cfg(debug_assertions)]
        if let Some(old_data) = old_value {
            eprintln!("impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
            eprintln!(
                "existing impl registered at {}:{}:{}",
                old_data.debug_info.file, old_data.debug_info.line, old_data.debug_info.column
            );
            eprintln!(
                "new impl registered at {}:{}:{}",
                entry.data.debug_info.file,
                entry.data.debug_info.line,
                entry.data.debug_info.column
            );
            panic!();
        }

        assert!(old_value.is_none(), "impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    fn data<T: TypeName + ?Sized>(&self, type_id: u64) -> Option<&ImplData> {
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

/// Guarantees that an impl has been registered for the type as the given trait
/// object.
#[doc(hidden)]
pub unsafe trait RegisteredImpl<T: ?Sized> {
    fn data() -> ImplData;
}

/// Registers a new impl with the trait object system.
///
/// This is called by `#[archive_dyn]` when attached to trait You might need to
/// do this if you're using generic traits and types, since each specific
/// instance needs to be individually registered.
///
/// Call it like `register_impl!(MyType as dyn MyTrait)`.
#[macro_export]
macro_rules! register_impl {
    ($type:ty as $trait:ty) => {
        const _: () = {
            use core::mem::MaybeUninit;
            use rkyv_dyn::{
                debug_info, inventory, validation, ImplData, ImplEntry, ImplValidation,
                RegisteredImpl, VTable,
            };

            unsafe impl RegisteredImpl<$trait> for $type {
                fn data() -> ImplData {
                    let vtable = unsafe {
                        // This is wildly unsafe but someone has to do it
                        let uninit = MaybeUninit::<$type>::uninit();
                        core::mem::transmute::<&$trait, (*const (), *const ())>(
                            core::mem::transmute::<*const $type, &$type>(uninit.as_ptr())
                                as &$trait,
                        )
                        .1
                    };

                    ImplData {
                        vtable: VTable(vtable),
                        debug_info: debug_info!(),
                        validation: validation!($type),
                    }
                }
            }

            inventory::submit! { ImplEntry::new::<$trait, $type>() }
        };
    };
}
