#[cfg(target_has_atomic = "ptr")]
mod atomic;

use core::alloc::LayoutError;

use ptr_meta::{from_raw_parts_mut, Pointee};
use rancor::{Fallible, Source};

use crate::{
    alloc::{alloc::alloc, boxed::Box, rc},
    de::{FromMetadata, Metadata, Pooling, PoolingExt as _, SharedPointer},
    rc::{ArchivedRc, ArchivedRcWeak, RcFlavor, RcResolver, RcWeakResolver},
    ser::{Sharing, Writer},
    traits::{ArchivePointee, LayoutRaw},
    Archive, ArchiveUnsized, Deserialize, DeserializeUnsized, Place, Serialize,
    SerializeUnsized,
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
    S::Error: Source,
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
    T::Metadata: Into<Metadata> + FromMetadata,
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
    S::Error: Source,
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
    T::Metadata: Into<Metadata> + FromMetadata,
    D: Fallible + Pooling + ?Sized,
    D::Error: Source,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<rc::Weak<T>, D::Error> {
        Ok(match self.upgrade() {
            None => rc::Weak::new(),
            Some(r) => rc::Rc::downgrade(&r.deserialize(deserializer)?),
        })
    }
}

#[cfg(test)]
mod tests {
    use munge::munge;
    use rancor::{Failure, Panic};

    use crate::{
        access_unchecked, access_unchecked_mut,
        alloc::{
            rc::{Rc, Weak},
            string::{String, ToString},
            vec,
        },
        api::{
            deserialize_using,
            test::{roundtrip, to_archived},
        },
        de::Pool,
        rc::{ArchivedRc, ArchivedRcWeak},
        to_bytes, Archive, Deserialize, Serialize,
    };

    #[test]
    fn roundtrip_rc() {
        #[derive(Debug, Eq, PartialEq, Archive, Deserialize, Serialize)]
        #[rkyv(crate, compare(PartialEq), derive(Debug))]
        struct Test {
            a: Rc<u32>,
            b: Rc<u32>,
        }

        let shared = Rc::new(10);
        let value = Test {
            a: shared.clone(),
            b: shared.clone(),
        };

        to_archived(&value, |mut archived| {
            assert_eq!(*archived, value);

            munge!(let ArchivedTest { a, .. } = archived.as_mut());
            unsafe {
                *ArchivedRc::get_seal_unchecked(a) = 42u32.into();
            }

            assert_eq!(*archived.a, 42);
            assert_eq!(*archived.b, 42);

            munge!(let ArchivedTest { b, .. } = archived.as_mut());
            unsafe {
                *ArchivedRc::get_seal_unchecked(b) = 17u32.into();
            }

            assert_eq!(*archived.a, 17);
            assert_eq!(*archived.b, 17);

            let mut deserializer = Pool::new();
            let deserialized = deserialize_using::<Test, _, Panic>(
                &*archived,
                &mut deserializer,
            )
            .unwrap();

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
        });
    }

    #[test]
    fn roundtrip_rc_zst() {
        #[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]
        #[rkyv(crate, compare(PartialEq), derive(Debug))]
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
    fn roundtrip_unsized_shared_ptr() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, compare(PartialEq), derive(Debug))]
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
    fn roundtrip_unsized_shared_ptr_empty() {
        #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
        #[rkyv(crate, compare(PartialEq), derive(Debug))]
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
    fn roundtrip_weak_ptr() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            a: Rc<u32>,
            b: Weak<u32>,
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

        munge!(let ArchivedTest { a, .. } = mutable_archived.as_mut());
        unsafe {
            *ArchivedRc::get_seal_unchecked(a) = 42u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 42);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 42);

        let mut mutable_archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(buf.as_mut()) };
        munge!(let ArchivedTest { b, .. } = mutable_archived.as_mut());
        unsafe {
            *ArchivedRc::get_seal_unchecked(
                ArchivedRcWeak::upgrade_seal(b).unwrap(),
            ) = 17u32.into();
        }

        let archived =
            unsafe { access_unchecked::<ArchivedTest>(buf.as_ref()) };
        assert_eq!(*archived.a, 17);
        assert!(archived.b.upgrade().is_some());
        assert_eq!(**archived.b.upgrade().unwrap(), 17);

        let mut deserializer = Pool::new();
        let deserialized =
            deserialize_using::<Test, _, Panic>(archived, &mut deserializer)
                .unwrap();

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

    #[test]
    fn serialize_cyclic_error() {
        use rancor::{Fallible, Source};

        use crate::{
            de::Pooling,
            ser::{Sharing, Writer},
        };

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(
            crate,
            serialize_bounds(
                __S: Sharing + Writer,
                <__S as Fallible>::Error: Source,
            ),
            deserialize_bounds(
                __D: Pooling,
                <__D as Fallible>::Error: Source,
            )
        )]
        #[cfg_attr(
            feature = "bytecheck",
            rkyv(bytecheck(bounds(
                __C: crate::validation::ArchiveContext
                    + crate::validation::SharedContext,
                <__C as Fallible>::Error: Source,
            ))),
        )]
        struct Inner {
            #[rkyv(omit_bounds)]
            weak: Weak<Self>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Outer {
            inner: Rc<Inner>,
        }

        let value = Outer {
            inner: Rc::new_cyclic(|weak| Inner { weak: weak.clone() }),
        };

        assert!(to_bytes::<Failure>(&value).is_err());
    }

    #[cfg(all(
        feature = "bytecheck",
        not(feature = "big_endian"),
        not(any(feature = "pointer_width_16", feature = "pointer_width_64")),
    ))]
    #[test]
    fn recursive_stack_overflow() {
        use rancor::{Fallible, Source};

        use crate::{
            access,
            de::Pooling,
            util::Align,
            validation::{ArchiveContext, SharedContext},
        };

        #[derive(Archive, Deserialize)]
        #[rkyv(
            crate,
            bytecheck(bounds(__C: ArchiveContext + SharedContext)),
            deserialize_bounds(
                __D: Pooling,
                <__D as Fallible>::Error: Source,
            ),
            derive(Debug),
        )]
        enum AllValues {
            Rc(#[rkyv(omit_bounds)] Rc<AllValues>),
        }

        let data = Align([
            0x00, 0x00, 0x00, 0xff, // B: AllValues::Rc
            0xfc, 0xff, 0xff, 0xff, // RelPtr with offset -4 (B)
            0x00, 0x00, 0xf6, 0xff, // A: AllValues::Rc
            0xf4, 0xff, 0xff, 0xff, // RelPtr with offset -12 (B)
        ]);
        access::<ArchivedAllValues, Failure>(&*data).unwrap_err();
    }
}
