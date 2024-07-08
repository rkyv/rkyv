//! Wrapper type support and commonly used wrappers.
//!
//! Wrappers can be applied with the `#[with(...)]` attribute in the
//! [`Archive`](macro@crate::Archive) macro.

// mod impls;

use core::marker::PhantomData;

use rancor::Fallible;

use crate::{Place, Portable};

/// A variant of [`Archive`](crate::Archive) that works with wrappers.
///
/// Creating a wrapper allows users to customize how fields are archived easily
/// without changing the unarchived type.
///
/// This trait allows wrapper types to transparently change the archive
/// behaviors for struct and enum fields. When a field is serialized, it may use
/// the implementations for the wrapper type and the given field instead of the
/// implementation for the type itself.
///
/// # Example
///
/// ```
/// use rkyv::{
///     access_unchecked, deserialize,
///     rancor::{Error, Fallible, Infallible, ResultExt as _},
///     to_bytes,
///     with::{ArchiveWith, DeserializeWith, SerializeWith},
///     Archive, Archived, Deserialize, Place, Resolver, Serialize,
/// };
///
/// struct Incremented;
///
/// impl ArchiveWith<i32> for Incremented {
///     type Archived = Archived<i32>;
///     type Resolver = Resolver<i32>;
///
///     fn resolve_with(field: &i32, _: (), out: Place<Self::Archived>) {
///         let incremented = field + 1;
///         incremented.resolve((), out);
///     }
/// }
///
/// impl<S> SerializeWith<i32, S> for Incremented
/// where
///     S: Fallible + ?Sized,
///     i32: Serialize<S>,
/// {
///     fn serialize_with(
///         field: &i32,
///         serializer: &mut S,
///     ) -> Result<Self::Resolver, S::Error> {
///         let incremented = field + 1;
///         incremented.serialize(serializer)
///     }
/// }
///
/// impl<D> DeserializeWith<Archived<i32>, i32, D> for Incremented
/// where
///     D: Fallible + ?Sized,
///     Archived<i32>: Deserialize<i32, D>,
/// {
///     fn deserialize_with(
///         field: &Archived<i32>,
///         deserializer: &mut D,
///     ) -> Result<i32, D::Error> {
///         Ok(field.deserialize(deserializer)? - 1)
///     }
/// }
///
/// #[derive(Archive, Deserialize, Serialize)]
/// struct Example {
///     #[with(Incremented)]
///     a: i32,
///     // Another i32 field, but not incremented this time
///     b: i32,
/// }
///
/// let value = Example { a: 4, b: 9 };
///
/// let buf = to_bytes::<Error>(&value).unwrap();
///
/// let archived =
///     unsafe { access_unchecked::<Archived<Example>>(buf.as_ref()) };
/// // The wrapped field has been incremented
/// assert_eq!(archived.a, 5);
/// // ... and the unwrapped field has not
/// assert_eq!(archived.b, 9);
///
/// let deserialized =
///     deserialize::<Example, _, Infallible>(archived, &mut ()).always_ok();
/// // The wrapped field is back to normal
/// assert_eq!(deserialized.a, 4);
/// // ... and the unwrapped field is unchanged
/// assert_eq!(deserialized.b, 9);
/// ```
pub trait ArchiveWith<F: ?Sized> {
    /// The archived type of `Self` with `F`.
    type Archived: Portable;
    /// The resolver of a `Self` with `F`.
    type Resolver;

    /// Resolves the archived type using a reference to the field type `F`.
    fn resolve_with(
        field: &F,
        resolver: Self::Resolver,
        out: Place<Self::Archived>,
    );
}

/// A variant of `Serialize` that works with `With` wrappers.
pub trait SerializeWith<F: ?Sized, S: Fallible + ?Sized>:
    ArchiveWith<F>
{
    /// Serializes the field type `F` using the given serializer.
    fn serialize_with(
        field: &F,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error>;
}

/// A variant of `Deserialize` that works with `With` wrappers.
pub trait DeserializeWith<F: ?Sized, T, D: Fallible + ?Sized> {
    /// Deserializes the field type `F` using the given deserializer.
    fn deserialize_with(field: &F, deserializer: &mut D)
        -> Result<T, D::Error>;
}

/// A generic wrapper that allows wrapping an `Option<T>`.
///
/// # Example
///
/// ```
/// use rkyv::{
///     with::{InlineAsBox, Map},
///     Archive,
/// };
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(Map<InlineAsBox>)]
///     option: Option<&'a i32>,
///     #[with(Map<InlineAsBox>)]
///     vec: Vec<&'a i32>,
/// }
/// ```
pub struct Map<T> {
    _phantom: PhantomData<T>,
}

/// A generic wrapper that allows wrapping a `HashMap<K, V>` or
/// `BTreeMap<K, V>`.
pub struct MapKV<K, V> {
    _phantom: PhantomData<(K, V)>,
}

/// A type indicating relaxed atomic loads.
pub struct Relaxed;

/// A type indicating acquire atomic loads.
pub struct Acquire;

/// A type indicating sequentially-consistent atomic loads.
pub struct SeqCst;

/// A wrapper that archives an atomic by loading its value with a particular
/// ordering.
///
/// When serializing, the specified ordering will be used to load the value from
/// the source atomic. The underlying archived type is still a non-atomic value.
///
/// See [`AsAtomic`] for an unsafe alternative which archives as an atomic.
///
/// # Example
///
/// ```
/// # #[cfg(target_has_atomic = "32")]
/// use core::sync::atomic::AtomicU32;
///
/// use rkyv::{
///     with::{AtomicLoad, Relaxed},
///     Archive,
/// };
///
/// # #[cfg(target_has_atomic = "32")]
/// #[derive(Archive)]
/// struct Example {
///     #[with(AtomicLoad<Relaxed>)]
///     a: AtomicU32,
/// }
/// ```
#[derive(Debug)]
pub struct AtomicLoad<SO> {
    _phantom: PhantomData<SO>,
}

/// A wrapper that archives an atomic with an underlying atomic.
///
/// When serializing and deserializing, the specified ordering will be used to
/// load the value from the source atomic.
///
/// See [`AtomicLoad`] for a safe alternative.
///
/// # Safety
///
/// This wrapper is only safe to use when the backing memory for wrapped types
/// is mutable.
///
/// # Example
///
/// ```
/// # #[cfg(target_has_atomic = "32")]
/// use core::sync::atomic::AtomicU32;
///
/// use rkyv::{
///     with::{AsAtomic, Relaxed},
///     Archive,
/// };
///
/// # #[cfg(target_has_atomic = "32")]
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsAtomic<Relaxed, Relaxed>)]
///     a: AtomicU32,
/// }
/// ```
#[derive(Debug)]
pub struct AsAtomic<SO, DO> {
    _phantom: PhantomData<(SO, DO)>,
}

/// A wrapper that serializes a reference inline.
///
/// References serialized with `Inline` cannot be deserialized because the
/// struct cannot own the deserialized value.
///
/// # Example
///
/// ```
/// use rkyv::{with::Inline, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(Inline)]
///     a: &'a i32,
/// }
/// ```
#[derive(Debug)]
pub struct Inline;

/// A wrapper that serializes a field into a box.
///
/// This functions similarly to [`AsInlineBox`], but is for regular fields
/// instead of references.
///
/// # Example
///
/// ```
/// use rkyv::{with::AsBox, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsBox)]
///     a: i32,
///     #[with(AsBox)]
///     b: str,
/// }
/// ```
#[derive(Debug)]
pub struct AsBox;

/// A wrapper that serializes a reference as if it were boxed.
///
/// Unlike [`Inline`], unsized references can be serialized with `InlineAsBox`.
///
/// References serialized with `InlineAsBox` cannot be deserialized because the
/// struct cannot own the deserialized value.
///
/// # Example
///
/// ```
/// use rkyv::{with::InlineAsBox, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(InlineAsBox)]
///     a: &'a i32,
///     #[with(InlineAsBox)]
///     b: &'a str,
/// }
/// ```
#[derive(Debug)]
pub struct InlineAsBox;

/// A wrapper that attempts to convert a type to and from UTF-8.
///
/// Types like `OsString` and `PathBuf` aren't guaranteed to be encoded as
/// UTF-8, but they usually are anyway. Using this wrapper will archive them as
/// if they were regular `String`s.
///
/// # Example
///
/// ```
/// use std::{ffi::OsString, path::PathBuf};
///
/// use rkyv::{with::AsString, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsString)]
///     os_string: OsString,
///     #[with(AsString)]
///     path: PathBuf,
/// }
/// ```
#[derive(Debug)]
pub struct AsString;

/// A wrapper that locks a lock and serializes the value immutably.
///
/// This wrapper can panic under very specific circumstances when:
///
/// 1. `serialize_with` is called and succeeds in locking the value to serialize
///    it.
/// 2. Another thread locks the value and panics, poisoning the lock
/// 3. `resolve_with` is called and gets a poisoned value.
///
/// Unfortunately, it's not possible to work around this issue. If your code
/// absolutely must not panic under any circumstances, it's recommended that you
/// lock your values and then serialize them while locked.
///
/// # Example
///
/// ```
/// use std::sync::Mutex;
///
/// use rkyv::{
///     with::{Lock, Unsafe},
///     Archive,
/// };
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(Lock<Unsafe>)]
///     a: Mutex<i32>,
/// }
/// ```
#[derive(Debug)]
pub struct Lock<T> {
    _phantom: PhantomData<T>,
}

/// A wrapper that serializes a `Cow` as if it were owned.
///
/// # Example
///
/// ```
/// use std::borrow::Cow;
///
/// use rkyv::{with::AsOwned, Archive};
///
/// #[derive(Archive)]
/// struct Example<'a> {
///     #[with(AsOwned)]
///     a: Cow<'a, str>,
/// }
/// ```
#[derive(Debug)]
pub struct AsOwned;

/// A wrapper that serializes associative containers as a `Vec` of key-value
/// pairs.
///
/// This provides faster serialization for containers like `HashMap` and
/// `BTreeMap` by serializing the key-value pairs directly instead of building a
/// data structure in the buffer.
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
///
/// use rkyv::{with::AsVec, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsVec)]
///     values: HashMap<String, u32>,
/// }
/// ```
#[derive(Debug)]
pub struct AsVec;

/// A wrapper that niches some type combinations.
///
/// A common type combination is `Option<Box<T>>`. By using a null pointer, the
/// archived version can save some space on-disk.
///
/// # Example
///
/// ```
/// use core::mem::size_of;
///
/// use rkyv::{with::Niche, Archive, Archived};
///
/// #[derive(Archive)]
/// struct BasicExample {
///     value: Option<Box<str>>,
/// }
///
/// #[derive(Archive)]
/// struct NichedExample {
///     #[with(Niche)]
///     value: Option<Box<str>>,
/// }
///
/// assert!(
///     size_of::<Archived<BasicExample>>()
///         > size_of::<Archived<NichedExample>>()
/// );
/// ```
#[derive(Debug)]
pub struct Niche;

/// A wrapper that converts a [`SystemTime`](::std::time::SystemTime) to a
/// [`Duration`](::std::time::Duration) since
/// [`UNIX_EPOCH`](::std::time::UNIX_EPOCH).
///
/// If the serialized time occurs before the UNIX epoch, serialization will
/// panic during `resolve`. The resulting archived time will be an
/// [`ArchivedDuration`](crate::time::ArchivedDuration) relative to the UNIX
/// epoch.
///
/// # Example
///
/// ```
/// use rkyv::{Archive, with::AsUnixTime};
/// use std::time::SystemTime;
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(AsUnixTime)]
///     time: SystemTime,
/// }
#[derive(Debug)]
pub struct AsUnixTime;

/// A wrapper that allows serialize-unsafe types to be serialized.
///
/// Types like `Cell` and `UnsafeCell` may contain serializable types, but have
/// unsafe access semantics due to interior mutability. They may be safe to
/// serialize, but only under conditions that rkyv is unable to guarantee.
///
/// This wrapper enables serializing these types, and places the burden of
/// verifying that their access semantics are used safely on the user.
///
/// # Safety
///
/// Using this wrapper on types with interior mutability can create races
/// conditions or allow access to data in an invalid state if access semantics
/// are not followed properly. During serialization, the data must not be
/// modified.
///
/// # Example
///
/// ```
/// use core::cell::{Cell, UnsafeCell};
///
/// use rkyv::{with::Unsafe, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(Unsafe)]
///     cell: Cell<String>,
///     #[with(Unsafe)]
///     unsafe_cell: UnsafeCell<String>,
/// }
/// ```
#[derive(Debug)]
pub struct Unsafe;

/// A wrapper that skips serializing a field.
///
/// Skipped fields must implement `Default` to be deserialized.
///
/// # Example
///
/// ```
/// use rkyv::{with::Skip, Archive};
///
/// #[derive(Archive)]
/// struct Example {
///     #[with(Skip)]
///     a: u32,
/// }
/// ```
#[derive(Debug)]
pub struct Skip;

/// A wrapper that clones the contents of `Arc` and `Rc` pointers.
#[derive(Debug)]
pub struct Unshare;

#[cfg(test)]
mod tests {
    use core::{convert::Infallible, str::FromStr};
    use std::borrow::Cow;

    use crate::{
        access_unchecked, access_unchecked_mut,
        de::Pool,
        deserialize,
        primitive::ArchivedU32,
        rancor::{Error, Fallible},
        ser::Writer,
        test::roundtrip,
        to_bytes,
        with::{
            ArchiveWith, AsAtomic, AsBox, AsOwned, AsVec, AtomicLoad,
            DeserializeWith, Inline, InlineAsBox, Niche, Relaxed,
            SerializeWith, Unsafe,
        },
        Archive, Archived, Deserialize, Place, Serialize,
    };

    struct ConvertToString;

    impl<T: ToString> ArchiveWith<T> for ConvertToString {
        type Archived = <String as Archive>::Archived;
        type Resolver = <String as Archive>::Resolver;

        fn resolve_with(
            value: &T,
            resolver: Self::Resolver,
            out: Place<Self::Archived>,
        ) {
            value.to_string().resolve(resolver, out);
        }
    }

    impl<T: ToString, S: Fallible + Writer + ?Sized> SerializeWith<T, S>
        for ConvertToString
    {
        fn serialize_with(
            value: &T,
            serializer: &mut S,
        ) -> Result<Self::Resolver, S::Error> {
            Ok(value.to_string().serialize(serializer)?)
        }
    }

    impl<T: FromStr, D: Fallible + ?Sized>
        DeserializeWith<Archived<String>, T, D> for ConvertToString
    where
        <T as FromStr>::Err: core::fmt::Debug,
    {
        fn deserialize_with(
            value: &Archived<String>,
            _: &mut D,
        ) -> Result<T, D::Error> {
            Ok(T::from_str(value.as_str()).unwrap())
        }
    }

    #[test]
    fn with_struct() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(ConvertToString)]
            value: i32,
            other: i32,
        }

        let value = Test {
            value: 10,
            other: 10,
        };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.value, "10");
        assert_eq!(archived.other, 10);

        let deserialized =
            deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
        assert_eq!(deserialized.value, 10);
        assert_eq!(deserialized.other, 10);
    }

    #[test]
    fn with_tuple_struct() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test(#[with(ConvertToString)] i32, i32);

        let value = Test(10, 10);
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.0, "10");
        assert_eq!(archived.1, 10);

        let deserialized =
            deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
        assert_eq!(deserialized.0, 10);
        assert_eq!(deserialized.1, 10);
    }

    #[test]
    fn with_enum() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        enum Test {
            A {
                #[with(ConvertToString)]
                value: i32,
                other: i32,
            },
            B(#[with(ConvertToString)] i32, i32),
        }

        let value = Test::A {
            value: 10,
            other: 10,
        };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        if let ArchivedTest::A { value, other } = archived {
            assert_eq!(*value, "10");
            assert_eq!(*other, 10);
        } else {
            panic!("expected variant A");
        };

        let deserialized =
            deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
        if let Test::A { value, other } = &deserialized {
            assert_eq!(*value, 10);
            assert_eq!(*other, 10);
        } else {
            panic!("expected variant A");
        };

        let value = Test::B(10, 10);
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        if let ArchivedTest::B(value, other) = archived {
            assert_eq!(*value, "10");
            assert_eq!(*other, 10);
        } else {
            panic!("expected variant B");
        };

        let deserialized =
            deserialize::<Test, _, Infallible>(archived, &mut ()).unwrap();
        if let Test::B(value, other) = &deserialized {
            assert_eq!(*value, 10);
            assert_eq!(*other, 10);
        } else {
            panic!("expected variant B");
        };
    }

    #[test]
    fn with_atomic_load() {
        use core::sync::atomic::{AtomicU32, Ordering};

        #[derive(Archive, Debug, Deserialize, Serialize)]
        #[rkyv(crate, check_bytes, derive(Debug))]
        struct Test {
            #[with(AtomicLoad<Relaxed>)]
            a: AtomicU32,
        }

        impl PartialEq for Test {
            fn eq(&self, other: &Self) -> bool {
                self.a.load(Ordering::Relaxed)
                    == other.a.load(Ordering::Relaxed)
            }
        }

        impl PartialEq<Test> for ArchivedTest {
            fn eq(&self, other: &Test) -> bool {
                self.a == other.a.load(Ordering::Relaxed)
            }
        }

        let value = Test {
            a: AtomicU32::new(42),
        };
        roundtrip(&value);
    }

    #[test]
    fn with_as_atomic() {
        use core::sync::atomic::{AtomicU32, Ordering};

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(AsAtomic<Relaxed, Relaxed>)]
            value: AtomicU32,
        }

        let value = Test {
            value: AtomicU32::new(42),
        };
        let mut bytes = to_bytes::<Error>(&value).unwrap();
        // NOTE: with(Atomic) is only sound if the backing memory is mutable,
        // use with caution!
        let archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(&mut bytes) };

        assert_eq!(archived.value.load(Ordering::Relaxed), 42);
    }

    #[test]
    fn with_inline() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[with(Inline)]
            value: &'a i32,
        }

        let a = 42;
        let value = Test { value: &a };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.value, 42);
    }

    #[test]
    fn with_boxed() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(AsBox)]
            value: i32,
        }

        let value = Test { value: 42 };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.value.get(), &42);
    }

    #[test]
    fn with_boxed_inline() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[with(InlineAsBox)]
            value: &'a str,
        }

        let a = "hello world";
        let value = Test { value: &a };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.value.as_ref(), "hello world");
    }

    #[test]
    fn with_as_owned() {
        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test<'a> {
            #[with(AsOwned)]
            a: Cow<'a, u32>,
            #[with(AsOwned)]
            b: Cow<'a, [u32]>,
            #[with(AsOwned)]
            c: Cow<'a, str>,
        }

        let value = Test {
            a: Cow::Borrowed(&100),
            b: Cow::Borrowed(&[1, 2, 3, 4, 5, 6]),
            c: Cow::Borrowed("hello world"),
        };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.a, 100);
        assert_eq!(archived.b, [1, 2, 3, 4, 5, 6]);
        assert_eq!(archived.c, "hello world");
    }

    #[test]
    fn with_as_vec() {
        #[cfg(not(feature = "std"))]
        use alloc::collections::{BTreeMap, BTreeSet};
        #[cfg(feature = "std")]
        use std::collections::{BTreeMap, BTreeSet};

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(AsVec)]
            a: BTreeMap<String, String>,
            #[with(AsVec)]
            b: BTreeSet<String>,
            #[with(AsVec)]
            c: BTreeMap<String, String>,
        }

        let mut a = BTreeMap::new();
        a.insert("foo".to_string(), "hello".to_string());
        a.insert("bar".to_string(), "world".to_string());
        a.insert("baz".to_string(), "bat".to_string());

        let mut b = BTreeSet::new();
        b.insert("foo".to_string());
        b.insert("hello world!".to_string());
        b.insert("bar".to_string());
        b.insert("fizzbuzz".to_string());

        let c = BTreeMap::new();

        let value = Test { a, b, c };

        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert_eq!(archived.a.len(), 3);
        assert!(archived
            .a
            .iter()
            .find(|&e| e.key == "foo" && e.value == "hello")
            .is_some());
        assert!(archived
            .a
            .iter()
            .find(|&e| e.key == "bar" && e.value == "world")
            .is_some());
        assert!(archived
            .a
            .iter()
            .find(|&e| e.key == "baz" && e.value == "bat")
            .is_some());

        assert_eq!(archived.b.len(), 4);
        assert!(archived.b.iter().find(|&e| e == "foo").is_some());
        assert!(archived.b.iter().find(|&e| e == "hello world!").is_some());
        assert!(archived.b.iter().find(|&e| e == "bar").is_some());
        assert!(archived.b.iter().find(|&e| e == "fizzbuzz").is_some());
    }

    #[test]
    fn with_niche() {
        use core::mem::size_of;

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(Niche)]
            inner: Option<Box<String>>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct TestNoNiching {
            inner: Option<Box<String>>,
        }

        let value = Test {
            inner: Some(Box::new("hello world".to_string())),
        };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert!(archived.inner.is_some());
        assert_eq!(&**archived.inner.as_ref().unwrap(), "hello world");
        assert_eq!(archived.inner, value.inner);

        let value = Test { inner: None };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert!(archived.inner.is_none());
        assert_eq!(archived.inner, value.inner);

        assert!(
            size_of::<Archived<Test>>() < size_of::<Archived<TestNoNiching>>()
        );
    }

    #[test]
    fn with_niche_nonzero() {
        use core::{
            mem::size_of,
            num::{
                NonZeroI32, NonZeroI8, NonZeroIsize, NonZeroU32, NonZeroU8,
                NonZeroUsize,
            },
        };

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(Niche)]
            a: Option<NonZeroI8>,
            #[with(Niche)]
            b: Option<NonZeroI32>,
            #[with(Niche)]
            c: Option<NonZeroIsize>,
            #[with(Niche)]
            d: Option<NonZeroU8>,
            #[with(Niche)]
            e: Option<NonZeroU32>,
            #[with(Niche)]
            f: Option<NonZeroUsize>,
        }

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct TestNoNiching {
            a: Option<NonZeroI8>,
            b: Option<NonZeroI32>,
            c: Option<NonZeroIsize>,
            d: Option<NonZeroU8>,
            e: Option<NonZeroU32>,
            f: Option<NonZeroUsize>,
        }

        let value = Test {
            a: Some(NonZeroI8::new(10).unwrap()),
            b: Some(NonZeroI32::new(10).unwrap()),
            c: Some(NonZeroIsize::new(10).unwrap()),
            d: Some(NonZeroU8::new(10).unwrap()),
            e: Some(NonZeroU32::new(10).unwrap()),
            f: Some(NonZeroUsize::new(10).unwrap()),
        };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert!(archived.a.is_some());
        assert_eq!(archived.a.as_ref().unwrap().get(), 10);
        assert!(archived.b.is_some());
        assert_eq!(archived.b.as_ref().unwrap().get(), 10);
        assert!(archived.c.is_some());
        assert_eq!(archived.c.as_ref().unwrap().get(), 10);
        assert!(archived.d.is_some());
        assert_eq!(archived.d.as_ref().unwrap().get(), 10);
        assert!(archived.e.is_some());
        assert_eq!(archived.e.as_ref().unwrap().get(), 10);
        assert!(archived.f.is_some());
        assert_eq!(archived.f.as_ref().unwrap().get(), 10);

        let value = Test {
            a: None,
            b: None,
            c: None,
            d: None,
            e: None,
            f: None,
        };
        let bytes = to_bytes::<Error>(&value).unwrap();
        let archived = unsafe { access_unchecked::<ArchivedTest>(&bytes) };

        assert!(archived.a.is_none());
        assert!(archived.b.is_none());
        assert!(archived.c.is_none());
        assert!(archived.d.is_none());
        assert!(archived.e.is_none());
        assert!(archived.f.is_none());

        assert!(
            size_of::<Archived<Test>>() < size_of::<Archived<TestNoNiching>>()
        );
    }

    #[test]
    fn with_unsafe() {
        use core::cell::UnsafeCell;

        #[derive(Archive, Serialize, Deserialize)]
        #[rkyv(crate)]
        struct Test {
            #[with(Unsafe)]
            inner: UnsafeCell<u32>,
        }

        let value = Test {
            inner: UnsafeCell::new(100),
        };
        let mut bytes = to_bytes::<Error>(&value).unwrap();
        let archived =
            unsafe { access_unchecked_mut::<ArchivedTest>(&mut bytes) };

        unsafe {
            assert_eq!(*archived.inner.get(), 100);
            *archived.inner.get() = ArchivedU32::from_native(42u32);
            assert_eq!(*archived.inner.get(), 42);
        }

        let deserialized =
            deserialize::<Test, _, Error>(&*archived, &mut Pool::new())
                .unwrap();

        unsafe {
            assert_eq!(*deserialized.inner.get(), 42);
            *deserialized.inner.get() = 88;
            assert_eq!(*deserialized.inner.get(), 88);
        }
    }
}
