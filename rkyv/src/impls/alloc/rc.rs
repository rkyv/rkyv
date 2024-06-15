#[cfg(not(feature = "std"))]
use alloc::{alloc::alloc, boxed::Box, rc, sync};
use core::alloc::LayoutError;
#[cfg(feature = "std")]
use std::{alloc::alloc, rc, sync};

use ptr_meta::{from_raw_parts_mut, Pointee};
use rancor::{Fallible, Source};

use crate::{
    de::{Metadata, Pooling, PoolingExt as _, SharedPointer},
    rc::{
        ArcFlavor, ArchivedRc, ArchivedRcWeak, RcFlavor, RcResolver,
        RcWeakResolver,
    },
    ser::{Sharing, Writer},
    Archive, ArchivePointee, ArchiveUnsized, Deserialize, DeserializeUnsized,
    LayoutRaw, Place, Serialize, SerializeUnsized,
};

// Rc

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Rc<T> {
    type Archived = ArchivedRc<T::Archived, RcFlavor>;
    type Resolver = RcResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedRc::resolve_from_ref(self.as_ref(), resolver, out);
    }
}

impl<T, S> Serialize<S> for rc::Rc<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived, RcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

unsafe impl<T: LayoutRaw + Pointee + ?Sized> SharedPointer<T> for rc::Rc<T> {
    fn alloc(metadata: T::Metadata) -> Result<*mut T, LayoutError> {
        let layout = T::layout_raw(metadata)?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc(layout) }
        } else {
            crate::polyfill::dangling(&layout).as_ptr()
        };
        let ptr = from_raw_parts_mut(data_address.cast(), metadata);
        Ok(ptr)
    }

    unsafe fn from_value(ptr: *mut T) -> *mut T {
        let rc = rc::Rc::<T>::from(unsafe { Box::from_raw(ptr) });
        rc::Rc::into_raw(rc).cast_mut()
    }

    unsafe fn drop(ptr: *mut T) {
        drop(unsafe { rc::Rc::from_raw(ptr) });
    }
}

impl<T, D> Deserialize<rc::Rc<T>, D> for ArchivedRc<T::Archived, RcFlavor>
where
    T: ArchiveUnsized + LayoutRaw + Pointee + ?Sized + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<rc::Rc<T>, D::Error> {
        let raw_shared_ptr =
            deserializer.deserialize_shared::<_, rc::Rc<T>>(self.get())?;
        unsafe {
            rc::Rc::<T>::increment_strong_count(raw_shared_ptr);
        }
        unsafe { Ok(rc::Rc::<T>::from_raw(raw_shared_ptr)) }
    }
}

impl<T, U> PartialEq<rc::Rc<U>> for ArchivedRc<T, RcFlavor>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    fn eq(&self, other: &rc::Rc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// rc::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for rc::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, RcFlavor>;
    type Resolver = RcWeakResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            resolver,
            out,
        );
    }
}

impl<T, S> Serialize<S> for rc::Weak<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::<T::Archived, RcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

impl<T, D> Deserialize<rc::Weak<T>, D> for ArchivedRcWeak<T::Archived, RcFlavor>
where
    // Deserialize can only be implemented for sized types because weak pointers
    // to unsized types don't have `new` functions.
    T: ArchiveUnsized
        + LayoutRaw
        + Pointee // + ?Sized
        + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<rc::Weak<T>, D::Error> {
        Ok(match self {
            ArchivedRcWeak::None => rc::Weak::new(),
            ArchivedRcWeak::Some(r) => {
                rc::Rc::downgrade(&r.deserialize(deserializer)?)
            }
        })
    }
}

// Arc

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Arc<T> {
    type Archived = ArchivedRc<T::Archived, ArcFlavor>;
    type Resolver = RcResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedRc::resolve_from_ref(self.as_ref(), resolver, out);
    }
}

impl<T, S> Serialize<S> for sync::Arc<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRc::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.as_ref(),
            serializer,
        )
    }
}

unsafe impl<T: LayoutRaw + Pointee + ?Sized> SharedPointer<T> for sync::Arc<T> {
    fn alloc(metadata: T::Metadata) -> Result<*mut T, LayoutError> {
        let layout = T::layout_raw(metadata)?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc(layout) }
        } else {
            crate::polyfill::dangling(&layout).as_ptr()
        };
        let ptr = from_raw_parts_mut(data_address.cast(), metadata);
        Ok(ptr)
    }

    unsafe fn from_value(ptr: *mut T) -> *mut T {
        let arc = sync::Arc::<T>::from(unsafe { Box::from_raw(ptr) });
        sync::Arc::into_raw(arc).cast_mut()
    }

    unsafe fn drop(ptr: *mut T) {
        drop(unsafe { sync::Arc::from_raw(ptr) });
    }
}

impl<T, D> Deserialize<sync::Arc<T>, D> for ArchivedRc<T::Archived, ArcFlavor>
where
    T: ArchiveUnsized + LayoutRaw + Pointee + ?Sized + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Arc<T>, D::Error> {
        let raw_shared_ptr =
            deserializer.deserialize_shared::<_, sync::Arc<T>>(self.get())?;
        unsafe {
            sync::Arc::<T>::increment_strong_count(raw_shared_ptr);
        }
        unsafe { Ok(sync::Arc::<T>::from_raw(raw_shared_ptr)) }
    }
}

impl<T, U> PartialEq<sync::Arc<U>> for ArchivedRc<T, ArcFlavor>
where
    T: ArchivePointee + PartialEq<U> + ?Sized,
    U: ?Sized,
{
    fn eq(&self, other: &sync::Arc<U>) -> bool {
        self.get().eq(other.as_ref())
    }
}

// sync::Weak

impl<T: ArchiveUnsized + ?Sized> Archive for sync::Weak<T> {
    type Archived = ArchivedRcWeak<T::Archived, ArcFlavor>;
    type Resolver = RcWeakResolver;

    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedRcWeak::resolve_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            resolver,
            out,
        );
    }
}

impl<T, S> Serialize<S> for sync::Weak<T>
where
    T: SerializeUnsized<S> + ?Sized + 'static,
    S: Fallible + Writer + Sharing + ?Sized,
{
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedRcWeak::<T::Archived, ArcFlavor>::serialize_from_ref(
            self.upgrade().as_ref().map(|v| v.as_ref()),
            serializer,
        )
    }
}

// Deserialize can only be implemented for sized types because weak pointers
// don't have from/into raw functions.
impl<T, D> Deserialize<sync::Weak<T>, D>
    for ArchivedRcWeak<T::Archived, ArcFlavor>
where
    // Deserialize can only be implemented for sized types because weak pointers
    // to unsized types don't have `new` functions.
    T: ArchiveUnsized
        + LayoutRaw
        + Pointee // + ?Sized
        + 'static,
    T::Archived: DeserializeUnsized<T, D>,
    T::Metadata: Into<Metadata>,
    Metadata: Into<T::Metadata>,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<sync::Weak<T>, D::Error> {
        Ok(match self {
            ArchivedRcWeak::None => sync::Weak::new(),
            ArchivedRcWeak::Some(r) => {
                sync::Arc::downgrade(&r.deserialize(deserializer)?)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    use rancor::Panic;

    use super::rc::{Rc, Weak};
    use crate::{
        access_unchecked, access_unchecked_mut, de::Pool, deserialize,
        test::roundtrip, to_bytes, Archive, Archived, Deserialize, Serialize,
    };

    #[test]
    fn roundtrip_rc() {
        #[derive(Debug, Eq, PartialEq, Archive, Deserialize, Serialize)]
        #[archive(crate, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: Rc<u32>,
            b: Rc<u32>,
        }

        impl ArchivedTest {
            fn a(self: Pin<&mut Self>) -> Pin<&mut Archived<Rc<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.a) }
            }

            fn b(self: Pin<&mut Self>) -> Pin<&mut Archived<Rc<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.b) }
            }
        }

        let shared = Rc::new(10);
        let value = Test {
            a: shared.clone(),
            b: shared.clone(),
        };

        let mut buf = to_bytes::<Panic>(&value).unwrap();

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(archived, &value);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_mut_unchecked() =
                42u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert_eq!(*archived.b, 42);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived.as_mut().b().get_pin_mut_unchecked() =
                17u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert_eq!(*archived.b, 17);

        let mut deserializer = Pool::new();
        let deserialized =
            deserialize::<Test, _, Panic>(archived, &mut deserializer).unwrap();

        assert_eq!(*deserialized.a, 17);
        assert_eq!(*deserialized.b, 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 3);
        assert_eq!(Rc::strong_count(&deserialized.b), 3);
        assert_eq!(Rc::weak_count(&deserialized.a), 0);
        assert_eq!(Rc::weak_count(&deserialized.b), 0);

        core::mem::drop(deserializer);

        assert_eq!(*deserialized.a, 17);
        assert_eq!(*deserialized.b, 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 2);
        assert_eq!(Rc::strong_count(&deserialized.b), 2);
        assert_eq!(Rc::weak_count(&deserialized.a), 0);
        assert_eq!(Rc::weak_count(&deserialized.b), 0);
    }

    #[test]
    fn roundtrip_rc_zst() {
        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[archive(crate, check_bytes, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct TestRcZST {
            a: Rc<()>,
            b: Rc<()>,
        }

        let rc_zst = Rc::new(());
        roundtrip(&TestRcZST {
            a: rc_zst.clone(),
            b: rc_zst.clone(),
        });
    }

    #[test]
    fn archive_unsized_shared_ptr() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(crate, check_bytes, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: Rc<[String]>,
            b: Rc<[String]>,
        }

        let rc_slice = Rc::<[String]>::from(
            vec!["hello".to_string(), "world".to_string()].into_boxed_slice(),
        );
        let value = Test {
            a: rc_slice.clone(),
            b: rc_slice,
        };

        roundtrip(&value);
    }

    #[test]
    fn archive_unsized_shared_ptr_empty() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[archive(crate, check_bytes, compare(PartialEq))]
        #[archive_attr(derive(Debug))]
        struct Test {
            a: Rc<[u32]>,
            b: Rc<[u32]>,
        }

        let a_rc_slice = Rc::<[u32]>::from(vec![].into_boxed_slice());
        let b_rc_slice = Rc::<[u32]>::from(vec![100].into_boxed_slice());
        let value = Test {
            a: a_rc_slice,
            b: b_rc_slice.clone(),
        };

        roundtrip(&value);
    }

    #[test]
    fn archive_weak_ptr() {
        #[derive(Archive, Serialize, Deserialize)]
        #[archive(crate, check_bytes)]
        struct Test {
            a: Rc<u32>,
            b: Weak<u32>,
        }

        impl ArchivedTest {
            fn a(self: Pin<&mut Self>) -> Pin<&mut Archived<Rc<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.a) }
            }

            fn b(self: Pin<&mut Self>) -> Pin<&mut Archived<Weak<u32>>> {
                unsafe { self.map_unchecked_mut(|s| &mut s.b) }
            }
        }

        let shared = Rc::new(10);
        let value = Test {
            a: shared.clone(),
            b: Rc::downgrade(&shared),
        };

        let mut buf = to_bytes::<Panic>(&value).unwrap();

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 10);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 10);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived.as_mut().a().get_pin_mut_unchecked() =
                42u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 42);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        unsafe {
            *mutable_archived
                .as_mut()
                .b()
                .upgrade_pin_mut()
                .unwrap()
                .get_pin_mut_unchecked() = 17u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 17);

        let mut deserializer = Pool::new();
        let deserialized =
            deserialize::<Test, _, Panic>(archived, &mut deserializer).unwrap();

        assert_eq!(*deserialized.a, 17);
        assert!(deserialized.b.upgrade().is_some());
        assert_eq!(*deserialized.b.upgrade().unwrap(), 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b.upgrade().unwrap() as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 2);
        assert_eq!(Weak::strong_count(&deserialized.b), 2);
        assert_eq!(Rc::weak_count(&deserialized.a), 1);
        assert_eq!(Weak::weak_count(&deserialized.b), 1);

        core::mem::drop(deserializer);

        assert_eq!(*deserialized.a, 17);
        assert!(deserialized.b.upgrade().is_some());
        assert_eq!(*deserialized.b.upgrade().unwrap(), 17);
        assert_eq!(
            &*deserialized.a as *const u32,
            &*deserialized.b.upgrade().unwrap() as *const u32
        );
        assert_eq!(Rc::strong_count(&deserialized.a), 1);
        assert_eq!(Weak::strong_count(&deserialized.b), 1);
        assert_eq!(Rc::weak_count(&deserialized.a), 1);
        assert_eq!(Weak::weak_count(&deserialized.b), 1);
    }
}
