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
use std::{
    collections::{
        hash_map::DefaultHasher,
        HashMap,
    },
};
use archive::{
    offset_of,
    RelPtr,
    Write,
};

pub use inventory;

pub type DynError = Box<dyn Any>;

pub trait DynWrite {
    fn pos(&self) -> usize;

    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError>;
}

pub struct DynWriter<'a, W: Write + ?Sized> {
    inner: &'a mut W,
}

impl<'a, W: Write + ?Sized> DynWriter<'a, W> {
    pub fn new(inner: &'a mut W) -> Self {
        Self {
            inner,
        }
    }
}

impl<'a, W: Write + ?Sized> DynWrite for DynWriter<'a, W> {
    fn pos(&self) -> usize {
        self.inner.pos()
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), DynError> {
        match self.inner.write(bytes) {
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

pub trait ArchiveDyn {
    fn archive_dyn(&self, writer: &mut dyn DynWrite) -> Result<DynResolver, DynError>;
}

pub struct DynResolver {
    pos: usize,
    id: ImplId,
}

impl DynResolver {
    pub fn new(pos: usize, id: ImplId) -> Self {
        Self {
            pos,
            id,
        }
    }
}

pub struct TraitObject(*const (), *const ());

pub struct ArchivedDyn<T: ?Sized> {
    ptr: RelPtr<()>,
    #[cfg(not(feature = "vtable_cache"))]
    id: u64,
    #[cfg(feature = "vtable_cache")]
    vtable: core::sync::atomic::AtomicU64,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> ArchivedDyn<T> {
    pub fn new(from: usize, resolver: DynResolver) -> ArchivedDyn<T> {
        ArchivedDyn {
            ptr: RelPtr::new(from + offset_of!(ArchivedDyn<T>, ptr), resolver.pos),
            #[cfg(not(feature = "vtable_cache"))]
            id: resolver.id.0,
            #[cfg(feature = "vtable_cache")]
            vtable: AtomicU64::new(resolver.id.0),
            _phantom: PhantomData,
        }
    }

    pub fn data_ptr(&self) -> *const () {
        self.ptr.as_ptr()
    }

    #[cfg(feature = "vtable_cache")]
    pub fn vtable(&self) -> *const () {
        let vtable = self.vtable.load(Ordering::Relaxed);
        if archive::likely(vtable & 1 == 0) {
            vtable as usize as *const ()
        } else {
            let ptr = TYPE_REGISTRY.get_vtable(ImplId(vtable));
            self.vtable.store(ptr as usize as u64, Ordering::Relaxed);
            ptr
        }
    }

    #[cfg(not(feature = "vtable_cache"))]
    pub fn vtable(&self) -> *const () {
        TYPE_REGISTRY.get_vtable(ImplId(self.id))
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
    pub fn new<T: Hash + ?Sized, U: Hash + ?Sized>(trait_name: &T, type_name: &U) -> Self {
        let mut hasher = DefaultHasher::new();
        trait_name.hash(&mut hasher);
        type_name.hash(&mut hasher);

        // The lowest significant bit of the impl id must be set so we can determine if a vtable
        // has been cached when the feature is enabled. This can't just be when the feature is on
        // so that impls have the same id across all builds.
        Self(hasher.finish() | 1)
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

#[macro_export]
macro_rules! vtable {
    ($type:ty, $trait:ty) => {
        (
            unsafe {
                core::mem::transmute::<$trait, (*const (), *const ())>(
                    core::mem::transmute::<*const $type, &$type>(
                        core::ptr::null::<$type>()
                    ) as $trait
                ).1
            }
        )
    }
}

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

    fn get_vtable(&self, id: ImplId) -> *const () {
        self.id_to_vtable.get(&id).expect("attempted to get vtable for an unregistered type").0
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
