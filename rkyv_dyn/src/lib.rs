//! Trait object serialization for rkyv.
//!
//! With rkyv_dyn, trait objects can be serialized with rkyv then the methods
//! can be called without deserializing. All it takes is some macro magic.
//!
//! See [`ArchiveDyn`] for an example of how to use rkyv_dyn.

#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

#[cfg(feature = "validation")]
pub mod validation;

use core::sync::atomic::{AtomicU64, Ordering};
use core::{
    any::Any,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};
use rkyv::{offset_of, Archive, RelPtr, Write, WriteExt};
use rkyv_typename::TypeName;
use std::collections::{hash_map::DefaultHasher, HashMap};

#[doc(hidden)]
pub use inventory;
pub use rkyv_dyn_derive::archive_dyn;
#[cfg(feature = "validation")]
pub use validation::VTableValidation;

#[cfg(all(feature = "vtable_cache", feature = "nightly"))]
use core::intrinsics::likely;
#[cfg(all(feature = "vtable_cache", not(feature = "nightly")))]
#[inline]
fn likely(b: bool) -> bool {
    b
}

/// A generic error that can be returned from a [`WriteDyn`].
pub type DynError = Box<dyn Any>;

/// An object-safe version of `Write`.
///
/// Instead of an associated error type, `WriteDyn` returns the [`DynError`]
/// type. If you have a writer that already implements `Write`, then it will
/// automatically implement `WriteDyn`.
pub trait WriteDyn {
    /// Returns the current position of the writer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the writer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError>;
}

impl<'a, W: Write + ?Sized> WriteDyn for &'a mut W {
    fn pos(&self) -> usize {
        Write::pos(*self)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError> {
        match Write::write(*self, bytes) {
            Ok(()) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }
}

impl<'a> Write for dyn WriteDyn + 'a {
    type Error = DynError;

    fn pos(&self) -> usize {
        <Self as WriteDyn>::pos(self)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        <Self as WriteDyn>::write(self, bytes)
    }
}

/// An object-safe version of `TypeName`.
///
/// This makes it possible to build the type name through a trait object.
pub trait TypeNameDyn {
    /// Submits the pieces of the type name to the given function.
    fn build_type_name(&self, f: &mut dyn FnMut(&str));
}

impl<T: TypeName> TypeNameDyn for T {
    fn build_type_name(&self, mut f: &mut dyn FnMut(&str)) {
        Self::build_type_name(&mut f);
    }
}

/// A trait object that can be serialized.
///
/// To add archive support for a trait object:
///
/// 1. Add [`archive_dyn`](macro@archive_dyn) on your trait to make an
/// archive-compatible version of it. By default, it will be named "Archive" +
/// your trait name.
/// 2. Implement `Archive` and `TypeName` for the types you want to make trait
/// objects of.
/// 3. Implement your trait for your type and add the attribute `#[archive_dyn]`
/// to it. Make sure to implement your trait for your archived type as well.
///
/// Then you're ready to serialize boxed trait objects!
///
/// Even though your unarchived values are boxed as archive trait objects, your
/// archived values are boxed as regular trait objects. This is because your
/// unarchived values have to implement `ArchiveDyn` but your archived values do
/// not.
///
/// ## Examples
///
/// See [`archive_dyn`](macro@archive_dyn) for customization options.
///
/// ```
/// use rkyv::{
///     Aligned,
///     Archive,
///     ArchiveBuffer,
///     Archived,
///     archived_value,
///     WriteExt,
/// };
/// use rkyv_dyn::archive_dyn;
/// use rkyv_typename::TypeName;
///
/// #[archive_dyn]
/// trait ExampleTrait {
///     fn value(&self) -> String;
/// }
///
/// #[derive(Archive, TypeName)]
/// struct StringStruct(String);
///
/// #[archive_dyn]
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
/// #[derive(Archive, TypeName)]
/// struct IntStruct(i32);
///
/// #[archive_dyn]
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
/// let boxed_int = Box::new(IntStruct(42)) as Box<dyn ArchiveExampleTrait>;
/// let boxed_string = Box::new(StringStruct("hello world".to_string())) as Box<dyn ArchiveExampleTrait>;
/// let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
/// let int_pos = writer.archive(&boxed_int)
///     .expect("failed to archive boxed int");
/// let string_pos = writer.archive(&boxed_string)
///     .expect("failed to archive boxed string");
/// let buf = writer.into_inner();
/// let archived_int = unsafe { archived_value::<Box<dyn ArchiveExampleTrait>, _>(&buf, int_pos) };
/// let archived_string = unsafe { archived_value::<Box<dyn ArchiveExampleTrait>, _>(&buf, string_pos) };
/// assert_eq!(archived_int.value(), "42");
/// assert_eq!(archived_string.value(), "hello world");
/// ```
pub trait ArchiveDyn: TypeNameDyn {
    /// Writes the value to the writer and returns a resolver that can create an
    /// [`ArchivedDyn`] reference.
    fn archive_dyn(&self, writer: &mut dyn WriteDyn) -> Result<DynResolver, DynError>;
}

impl<T: Archive + TypeName> ArchiveDyn for T {
    fn archive_dyn(&self, writer: &mut dyn WriteDyn) -> Result<DynResolver, DynError> {
        Ok(DynResolver::new(writer.archive(self)?))
    }
}

/// The resolver for an [`ArchivedDyn`].
pub struct DynResolver {
    pos: usize,
}

impl DynResolver {
    /// Creates a new `DynResolver` with a given data position.
    pub fn new(pos: usize) -> Self {
        Self { pos }
    }
}

#[doc(hidden)]
pub struct TraitObject(*const (), *const ());

/// A reference to an archived trait object.
///
/// This is essentially a pair of a data pointer and a vtable id. The
/// `vtable_cache` feature is recommended if your situation allows for it. With
/// `vtable_cache`, the vtable will only be looked up once and then stored
/// locally for subsequent lookups when the reference is dereferenced.
///
/// `ArchivedDyn` is the trait object extension of `RelPtr`.
#[derive(Debug)]
pub struct ArchivedDyn<T: ?Sized> {
    ptr: RelPtr,
    vtable: AtomicU64,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> ArchivedDyn<T> {
    /// Creates a new `ArchivedDyn` from a data position, [`DynResolver`], and
    /// an implementation id.
    pub fn new(from: usize, resolver: DynResolver, id: &ImplId) -> ArchivedDyn<T> {
        ArchivedDyn {
            ptr: unsafe { RelPtr::new(from + offset_of!(ArchivedDyn<T>, ptr), resolver.pos) },
            vtable: AtomicU64::new(id.0),
            _phantom: PhantomData,
        }
    }

    /// Gets the data pointer of the trait object.
    pub fn data_ptr(&self) -> *const () {
        self.ptr.as_ptr()
    }

    /// Gets the vtable pointer for this trait object. With the `vtable_cache`
    /// feature, this will store the vtable locally on the first lookup.
    pub fn vtable(&self) -> *const () {
        let vtable = self.vtable.load(Ordering::Relaxed);

        #[cfg(feature = "vtable_cache")]
        if likely(vtable & 1 == 0) {
            return vtable as usize as *const ();
        }

        let ptr = TYPE_REGISTRY
            .data(ImplId(vtable))
            .expect("attempted to get vtable for an unregistered type")
            .vtable
            .0;

        #[cfg(feature = "vtable_cache")]
        self.vtable.store(ptr as usize as u64, Ordering::Relaxed);

        ptr
    }
}

impl<T: ?Sized> Deref for ArchivedDyn<T>
where
    for<'a> &'a T: From<TraitObject>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        TraitObject(self.data_ptr(), self.vtable()).into()
    }
}

impl<T: ?Sized> DerefMut for ArchivedDyn<T>
where
    for<'a> &'a T: From<TraitObject>,
    for<'a> &'a mut T: From<TraitObject>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        TraitObject(self.data_ptr(), self.vtable()).into()
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ImplId(u64);

impl ImplId {
    fn from_hasher<H: Hasher>(hasher: H) -> Self {
        // The lowest significant bit of the impl id must be set so we can
        // determine if a vtable has been cached when the feature is enabled.
        // This can't just be when the feature is on so that impls have the same
        // id across all builds.
        Self(hasher.finish() | 1)
    }

    pub fn resolve<T: ArchiveDyn + TypeName + ?Sized>(archive_dyn: &T) -> Self {
        let mut hasher = DefaultHasher::new();
        <T as TypeName>::build_type_name(|piece| piece.hash(&mut hasher));
        archive_dyn.build_type_name(&mut |piece| piece.hash(&mut hasher));
        let result = Self::from_hasher(hasher);
        #[cfg(debug_assertions)]
        if TYPE_REGISTRY.data(result).is_none() {
            let mut trait_name = String::new();
            <T as TypeName>::build_type_name(|piece| trait_name += piece);
            let mut type_name = String::new();
            archive_dyn.build_type_name(&mut |piece| type_name += piece);
            panic!("attempted to resolve an unregistered vtable ({} as {}); if this type is generic, you may be missing an explicit register_vtable! for the type", type_name, trait_name);
        }
        result
    }

    pub fn register<TR: TypeName + ?Sized, TY: TypeName + ?Sized>() -> Self {
        let mut hasher = DefaultHasher::new();
        TR::build_type_name(|piece| piece.hash(&mut hasher));
        TY::build_type_name(|piece| piece.hash(&mut hasher));
        Self::from_hasher(hasher)
    }
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct VTableDebugInfo {
    pub file: &'static str,
    pub line: u32,
    pub column: u32,
}

#[cfg(debug_assertions)]
#[doc(hidden)]
#[macro_export]
macro_rules! debug_info {
    () => {
        rkyv_dyn::VTableDebugInfo {
            file: core::file!(),
            line: core::line!(),
            column: core::column!(),
        }
    };
}

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct VTableDebugInfo;

#[cfg(not(debug_assertions))]
#[doc(hidden)]
#[macro_export]
macro_rules! debug_info {
    () => {
        rkyv_dyn::VTableDebugInfo
    };
}

#[cfg(not(feature = "validation"))]
#[doc(hidden)]
#[derive(Copy, Clone)]
pub struct VTableValidation;

#[cfg(not(feature = "validation"))]
#[doc(hidden)]
#[macro_export]
macro_rules! validation {
    ($type:ty) => {
        VTableValidation
    };
}

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct VTableData {
    pub vtable: VTable,
    pub debug_info: VTableDebugInfo,
    pub validation: VTableValidation,
}

#[doc(hidden)]
pub struct ImplVTable {
    id: ImplId,
    data: VTableData,
}

impl ImplVTable {
    pub fn new(id: ImplId, data: VTableData) -> Self {
        Self { id, data }
    }
}

inventory::collect!(ImplVTable);

#[doc(hidden)]
#[derive(Clone, Copy)]
pub struct VTable(pub *const ());

impl From<*const ()> for VTable {
    fn from(vtable: *const ()) -> Self {
        Self(vtable)
    }
}

unsafe impl Send for VTable {}
unsafe impl Sync for VTable {}

struct TypeRegistry {
    id_to_vtable: HashMap<ImplId, VTableData>,
}

impl TypeRegistry {
    fn new() -> Self {
        Self {
            id_to_vtable: HashMap::new(),
        }
    }

    fn add_impl(&mut self, impl_vtable: &ImplVTable) {
        #[cfg(feature = "vtable_cache")]
        debug_assert!(
            (impl_vtable.data.vtable.0 as usize) & 1 == 0,
            "vtable has a non-zero least significant bit which breaks vtable caching"
        );
        let old_value = self
            .id_to_vtable
            .insert(impl_vtable.id, impl_vtable.data);

        #[cfg(debug_assertions)]
        if let Some(old_data) = old_value {
            eprintln!("impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
            eprintln!(
                "existing impl registered at {}:{}:{}",
                old_data.debug_info.file, old_data.debug_info.line, old_data.debug_info.column
            );
            eprintln!(
                "new impl registered at {}:{}:{}",
                impl_vtable.data.debug_info.file,
                impl_vtable.data.debug_info.line,
                impl_vtable.data.debug_info.column
            );
            panic!();
        }

        assert!(old_value.is_none(), "impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    fn data(&self, id: ImplId) -> Option<&VTableData> {
        self.id_to_vtable.get(&id)
    }
}

lazy_static::lazy_static! {
    static ref TYPE_REGISTRY: TypeRegistry = {
        let mut result = TypeRegistry::new();
        for impl_vtable in inventory::iter::<ImplVTable> {
            result.add_impl(impl_vtable);
        }
        result
    };
}

/// Registers a new vtable with the trait object system.
///
/// This is called by `#[archive_dyn]` when attached to trait You might need to
/// do this if you're using generic traits and types, since each specific
/// instance needs to be individually registered.
///
/// Call it like `register_vtable!(MyType as dyn MyTrait)`.
#[macro_export]
macro_rules! register_vtable {
    ($type:ty as $trait:ty) => {
        const _: () = {
            use rkyv::Archived;
            use rkyv_dyn::{
                debug_info, inventory, validation, ImplId, ImplVTable, VTableData, VTableValidation,
            };

            inventory::submit! {
                // This is wildly unsafe but someone has to do it
                let vtable = unsafe {
                    let uninit = core::mem::MaybeUninit::<Archived<$type>>::uninit();

                    core::mem::transmute::<&$trait, (*const (), *const ())>(
                        core::mem::transmute::<*const Archived<$type>, &Archived<$type>>(
                            uninit.as_ptr()
                        ) as &$trait
                    ).1
                };
                ImplVTable::new(
                    ImplId::register::<$trait, $type>(),
                    VTableData {
                        vtable: vtable.into(),
                        debug_info: debug_info!(),
                        validation: validation!($type),
                    }
                )
            }
        };
    };
}
