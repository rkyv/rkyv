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
pub use inventory;

#[cfg(all(feature = "vtable_cache", feature = "nightly"))]
use core::intrinsics::likely;
#[cfg(all(feature = "vtable_cache", not(feature = "nightly")))]
#[inline]
fn likely(b: bool) -> bool {
    b
}

pub type DynError = Box<dyn Any>;

pub trait DynWrite {
    fn pos(&self) -> usize;

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

pub trait TypeNameDyn {
    fn build_type_name(&self, f: &mut dyn FnMut(&str));
}

impl<T: TypeName> TypeNameDyn for T {
    fn build_type_name(&self, mut f: &mut dyn FnMut(&str)) {
        Self::build_type_name(&mut f);
    }
}

pub trait ArchiveDyn: TypeNameDyn {
    fn archive_dyn(&self, writer: &mut dyn DynWrite) -> Result<DynResolver, DynError>;
}

impl<T: Archive + TypeName> ArchiveDyn for T {
    fn archive_dyn(&self, writer: &mut dyn DynWrite) -> Result<DynResolver, DynError> {
        Ok(DynResolver::new(writer.archive(self)?))
    }
}

pub struct DynResolver {
    pos: usize,
}

impl DynResolver {
    pub fn new(pos: usize) -> Self {
        Self {
            pos,
        }
    }
}

pub struct TraitObject(*const (), *const ());

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
