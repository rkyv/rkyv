//! Trait object serialization for rkyv.
//!
//! With rkyv_dyn, trait objects can be serialized with rkyv then
//! the methods can be called without deserializing. All it takes
//! is some macro magic.
//!
//! See [`ArchiveDyn`] for an example of how to use rkyv_dyn.

#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

use core::{
    any::Any,
    hash::{
        Hash,
        Hasher,
    },
    marker::PhantomData,
    ops::Deref,
};
#[cfg(feature = "vtable_cache")]
use core::sync::atomic::{
    AtomicU64,
    Ordering,
};
use std::collections::{
    hash_map::DefaultHasher,
    HashMap,
};
use rkyv::{
    Archive,
    offset_of,
    RelPtr,
    Write,
    WriteExt,
};
use rkyv_typename::TypeName;

pub use rkyv_dyn_derive::archive_dyn;
#[doc(hidden)]
pub use inventory;

#[cfg(all(feature = "vtable_cache", feature = "nightly"))]
use core::intrinsics::likely;
#[cfg(all(feature = "vtable_cache", not(feature = "nightly")))]
#[inline]
fn likely(b: bool) -> bool {
    b
}

/// A generic error that can be returned from a [`DynWrite`].
pub type DynError = Box<dyn Any>;

/// An object-safe version of `Write`.
///
/// Instead of an associated error type, `DynWrite` returns the
/// [`DynError`] type. If you have a writer that already implements
/// `Write`, then it will automatically implement `DynWrite`.
pub trait DynWrite {
    /// Returns the current position of the writer.
    fn pos(&self) -> usize;

    /// Attempts to write the given bytes to the writer.
    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError>;
}

impl<'a, W: Write + ?Sized> DynWrite for &'a mut W {
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

impl<'a> Write for dyn DynWrite + 'a {
    type Error = DynError;

    fn pos(&self) -> usize {
        <Self as DynWrite>::pos(self)
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), Self::Error> {
        <Self as DynWrite>::write(self, bytes)
    }
}

/// An object-safe version of `TypeName`.
///
/// This makes it possible to build the type name through a
/// trait object.
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
/// 1. Add [`archive_dyn`](macro@archive_dyn) on your trait to
/// make an archive-compatible version of it. By default, it
/// will be named "Archive" + your trait name.
/// 2. Implement `Archive` and `TypeName` for the types you want
/// to make trait objects of.
/// 3. Implement your trait for your type and add the attribute
/// `#[archive_dyn]` to it. Make sure to implement your trait
/// for your archived type as well.
///
/// Then you're ready to serialize boxed trait objects!
///
/// Even though your unarchived values are boxed as archive
/// trait objects, your archived values are boxed as regular
/// trait objects. This is because your unarchived values have
/// to implement `ArchiveDyn` but your archived values do not.
///
/// ## Examples
///
/// See [`archive_dyn`](macro@archive_dyn) for customization
/// options.
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
/// fn main() {
///     let boxed_int = Box::new(IntStruct(42)) as Box<dyn ArchiveExampleTrait>;
///     let boxed_string = Box::new(StringStruct("hello world".to_string())) as Box<dyn ArchiveExampleTrait>;
///     let mut writer = ArchiveBuffer::new(Aligned([0u8; 256]));
///     let int_pos = writer.archive(&boxed_int)
///         .expect("failed to archive boxed int");
///     let string_pos = writer.archive(&boxed_string)
///         .expect("failed to archive boxed string");
///     let buf = writer.into_inner();
///     let archived_int = unsafe { archived_value::<Box<dyn ArchiveExampleTrait>>(buf.as_ref(), int_pos) };
///     let archived_string = unsafe { archived_value::<Box<dyn ArchiveExampleTrait>>(buf.as_ref(), string_pos) };
///     assert_eq!(archived_int.value(), "42");
///     assert_eq!(archived_string.value(), "hello world");
/// }
/// ```
pub trait ArchiveDyn: TypeNameDyn {
    /// Writes the value to the writer and returns a resolver
    /// that can create a [`ArchivedDyn`] reference.
    fn archive_dyn(&self, writer: &mut dyn DynWrite) -> Result<DynResolver, DynError>;
}

impl<T: Archive + TypeName> ArchiveDyn for T {
    fn archive_dyn(&self, writer: &mut dyn DynWrite) -> Result<DynResolver, DynError> {
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
        Self {
            pos,
        }
    }
}

#[doc(hidden)]
pub struct TraitObject(*const (), *const ());

/// A reference to an archived trait object.
///
/// This is essentially a pair of a data pointer and a vtable
/// id. The `vtable_cache` feature is recommended if your
/// situation allows for it. With `vtable_cache`, the
/// vtable will only be looked up once and then stored locally
/// for subsequent lookups when the reference is dereferenced.
///
/// `ArchivedDyn` is the trait object extension of `RelPtr`.
#[derive(Debug)]
pub struct ArchivedDyn<T: ?Sized> {
    ptr: RelPtr<()>,
    #[cfg(not(feature = "vtable_cache"))]
    id: u64,
    #[cfg(feature = "vtable_cache")]
    vtable: core::sync::atomic::AtomicU64,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> ArchivedDyn<T> {
    /// Creates a new `ArchivedDyn` from a data position,
    /// [`DynResolver`], and an implementation id.
    pub fn new(from: usize, resolver: DynResolver, id: &ImplId) -> ArchivedDyn<T> {
        ArchivedDyn {
            ptr: RelPtr::new(from + offset_of!(ArchivedDyn<T>, ptr), resolver.pos),
            #[cfg(not(feature = "vtable_cache"))]
            id: id.0,
            #[cfg(feature = "vtable_cache")]
            vtable: AtomicU64::new(id.0),
            _phantom: PhantomData,
        }
    }

    /// Gets the data pointer of the trait object.
    pub fn data_ptr(&self) -> *const () {
        self.ptr.as_ptr()
    }

    #[cfg(feature = "vtable_cache")]
    pub fn vtable(&self) -> *const () {
        let vtable = self.vtable.load(Ordering::Relaxed);
        if likely(vtable & 1 == 0) {
            vtable as usize as *const ()
        } else {
            let ptr = TYPE_REGISTRY.vtable(ImplId(vtable)).expect("attempted to get vtable for an unregistered type");
            self.vtable.store(ptr as usize as u64, Ordering::Relaxed);
            ptr
        }
    }

    /// Gets the vtable pointer for this trait object. With the
    /// `vtable_cache` feature, this will store the vtable
    /// locally on the first lookup.
    #[cfg(not(feature = "vtable_cache"))]
    pub fn vtable(&self) -> *const () {
        TYPE_REGISTRY.vtable(ImplId(self.id)).expect("attempted to get vtable for an unregistered type")
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

#[doc(hidden)]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct ImplId(u64);

impl ImplId {
    fn from_hasher<H: Hasher>(hasher: H) -> Self {
        // The lowest significant bit of the impl id must be set so we can determine if a vtable
        // has been cached when the feature is enabled. This can't just be when the feature is on
        // so that impls have the same id across all builds.
        Self(hasher.finish() | 1)
    }

    pub fn resolve<T: ArchiveDyn + TypeName + ?Sized>(archive_dyn: &T) -> Self {
        let mut hasher = DefaultHasher::new();
        <T as TypeName>::build_type_name(|piece| piece.hash(&mut hasher));
        archive_dyn.build_type_name(&mut |piece| piece.hash(&mut hasher));
        let result = Self::from_hasher(hasher);
        #[cfg(debug_assertions)]
        if TYPE_REGISTRY.vtable(result).is_none() {
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

#[doc(hidden)]
pub struct ImplVTable {
    id: ImplId,
    vtable: VTable,
}

impl ImplVTable {
    pub fn new(id: ImplId, vtable: VTable) -> Self {
        Self {
            id,
            vtable,
        }
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
    id_to_vtable: HashMap<ImplId, VTable>,
}

impl TypeRegistry {
    fn new() -> Self {
        Self {
            id_to_vtable: HashMap::new(),
        }
    }

    fn add_impl(&mut self, impl_vtable: &ImplVTable) {
        #[cfg(feature = "vtable_cache")]
        debug_assert!((impl_vtable.vtable.0 as usize) & 1 == 0, "vtable has a non-zero least significant bit which breaks vtable caching");
        let old_value = self.id_to_vtable.insert(impl_vtable.id, impl_vtable.vtable);
        debug_assert!(old_value.is_none(), "impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    fn vtable(&self, id: ImplId) -> Option<*const ()> {
        self.id_to_vtable.get(&id).map(|v| v.0)
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
/// This is called by `#[archive_dyn]` when attached to trait
/// impls, but can be called manually to register impls instead.
/// You might need to do this if you're using generic traits
/// and types, since each specific instance needs to be
/// individually registered.
///
/// Call it like `register_vtable!(MyType as dyn MyTrait)`.
#[macro_export]
macro_rules! register_vtable {
    ($type:ty as $trait:ty) => {
        const _: () = {
            use rkyv::Archived;
            use rkyv_dyn::{
                ImplId,
                ImplVTable,
                inventory,
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
                    vtable.into()
                )
            }
        };
    }
}
