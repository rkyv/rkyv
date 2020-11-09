#![cfg_attr(feature = "nightly", feature(core_intrinsics))]

use core::{
    any::{
        Any,
        TypeId,
    },
    hash::{
        Hash,
        Hasher,
    },
    marker::PhantomData,
    ops::Deref,
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
    key: ArchiveDynImpl,
}

impl DynResolver {
    pub fn new(pos: usize, key: ArchiveDynImpl) -> Self {
        Self {
            pos,
            key,
        }
    }
}

pub struct TraitObject(*const (), *const ());

#[cfg(feature = "vtable_cache")]
#[derive(Clone, Copy)]
union CachedVTable {
    int: u64,
    ptr: *const (),
}

pub struct ArchivedDyn<T: ?Sized> {
    ptr: RelPtr<()>,
    impl_id: u64,
    #[cfg(feature = "vtable_cache")]
    vtable_cache: core::cell::Cell<CachedVTable>,
    _phantom: PhantomData<T>,
}

impl<T: ?Sized> ArchivedDyn<T> {
    pub fn new(from: usize, resolver: DynResolver) -> ArchivedDyn<T> {
        ArchivedDyn {
            ptr: RelPtr::new(from + offset_of!(ArchivedDyn<T>, ptr), resolver.pos),
            impl_id: TypeRegistry::impl_id(&resolver.key),
            #[cfg(feature = "vtable_cache")]
            vtable_cache: core::cell::Cell::new(CachedVTable { int: 0 }),
            _phantom: PhantomData,
        }
    }

    pub fn data_ptr(&self) -> *const () {
        self.ptr.as_ptr()
    }

    #[cfg(feature = "vtable_cache")]
    pub fn vtable(&self) -> *const () {
        let cache = self.vtable_cache.get();
        if archive::likely(unsafe { cache.int } != 0) {
            unsafe { cache.ptr }
        } else {
            let vtable = TYPE_REGISTRY.get_vtable(self.impl_id);
            self.vtable_cache.set(CachedVTable { ptr: vtable });
            vtable
        }
    }

    #[cfg(not(feature = "vtable_cache"))]
    pub fn vtable(&self) -> *const () {
        TYPE_REGISTRY.get_vtable(self.impl_id)
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

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ArchiveDynImpl(&'static str, TypeId);

impl ArchiveDynImpl {
    pub fn new<T: 'static>(trait_name: &'static str) -> Self {
        Self(trait_name, TypeId::of::<T>())
    }
}

pub struct ArchiveDynImplVTable(ArchiveDynImpl, VTable);

impl ArchiveDynImplVTable {
    pub fn new(key: ArchiveDynImpl, vtable: VTable) -> Self {
        Self(key, vtable)
    }
}

inventory::collect!(ArchiveDynImplVTable);

#[macro_export]
macro_rules! vtable {
    ($type:ty, $trait:tt) => {
        (
            unsafe {
                core::mem::transmute::<&dyn $trait, (*const (), *const ())>(
                    core::mem::transmute::<*const $type, &$type>(
                        core::ptr::null::<$type>()
                    ) as &dyn $trait
                ).1
            }
        )
    }
}

#[derive(Clone)]
pub struct VTable(pub *const ());

impl From<*const ()> for VTable {
    fn from(vtable: *const ()) -> Self {
        Self(vtable)
    }
}

unsafe impl Send for VTable {}
unsafe impl Sync for VTable {}

struct TypeRegistry {
    id_to_vtable: HashMap<u64, VTable>,
}

impl TypeRegistry {
    fn new() -> Self {
        Self {
            id_to_vtable: HashMap::new(),
        }
    }

    fn add_impl(&mut self, impl_vtable: &ArchiveDynImplVTable) {
        let id = Self::impl_id(&impl_vtable.0);
        let old_value = self.id_to_vtable.insert(id, impl_vtable.1.clone());
        debug_assert!(old_value.is_none(), "impl id conflict, a trait implementation was likely added twice (but it's possible there was a hash collision)");
    }

    fn impl_id(key: &ArchiveDynImpl) -> u64 {
        let mut hasher = DefaultHasher::new();
        key.0.hash(&mut hasher);
        hasher.finish()
    }

    fn get_vtable(&self, id: u64) -> *const () {
        self.id_to_vtable.get(&id).expect("attempted to get vtable for an unregistered type").0
    }
}

lazy_static::lazy_static! {
    static ref TYPE_REGISTRY: TypeRegistry = {
        let mut result = TypeRegistry::new();
        for impl_vtable in inventory::iter::<ArchiveDynImplVTable> {
            result.add_impl(impl_vtable);
        }
        result
    };
}
