#[cfg(not(feature = "std"))]
use alloc::{
    borrow::Cow,
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
    vec::Vec,
};
use core::marker::PhantomData;
#[cfg(feature = "std")]
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
    sync::Arc,
};

use ptr_meta::Pointee;
use rancor::{Error, Fallible};

use crate::{
    collections::util::Entry,
    niche::option_box::{ArchivedOptionBox, OptionBoxResolver},
    ser::{Allocator, Writer},
    string::{ArchivedString, StringResolver},
    vec::{ArchivedVec, VecResolver},
    with::{
        ArchiveWith, AsOwned, AsVec, Cloned, DeserializeWith, Map, Niche,
        SerializeWith,
    },
    Archive, ArchiveUnsized, ArchivedMetadata, Deserialize, DeserializeUnsized,
    LayoutRaw, Serialize, SerializeUnsized,
};

// Map for Vecs

impl<A, O> ArchiveWith<Vec<O>> for Map<A>
where
    A: ArchiveWith<O>,
{
    type Archived = ArchivedVec<<A as ArchiveWith<O>>::Archived>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        field: &Vec<O>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(field.len(), pos, resolver, out)
    }
}

impl<A, O, S> SerializeWith<Vec<O>, S> for Map<A>
where
    S: Fallible + Allocator + Writer + ?Sized,
    A: ArchiveWith<O> + SerializeWith<O, S>,
{
    fn serialize_with(
        field: &Vec<O>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        // Wrapper for O so that we have an Archive and Serialize implementation
        // and ArchivedVec::serialize_from_* is happy about the bound
        // constraints
        struct RefWrapper<'o, A, O>(&'o O, PhantomData<A>);

        impl<A: ArchiveWith<O>, O> Archive for RefWrapper<'_, A, O> {
            type Archived = <A as ArchiveWith<O>>::Archived;
            type Resolver = <A as ArchiveWith<O>>::Resolver;

            unsafe fn resolve(
                &self,
                pos: usize,
                resolver: Self::Resolver,
                out: *mut Self::Archived,
            ) {
                A::resolve_with(self.0, pos, resolver, out)
            }
        }

        impl<A, O, S> Serialize<S> for RefWrapper<'_, A, O>
        where
            A: ArchiveWith<O> + SerializeWith<O, S>,
            S: Fallible + Writer + ?Sized,
        {
            fn serialize(&self, s: &mut S) -> Result<Self::Resolver, S::Error> {
                A::serialize_with(self.0, s)
            }
        }

        let iter = field
            .iter()
            .map(|value| RefWrapper::<'_, A, O>(value, PhantomData));

        ArchivedVec::serialize_from_iter(iter, s)
    }
}

impl<A, O, D>
    DeserializeWith<ArchivedVec<<A as ArchiveWith<O>>::Archived>, Vec<O>, D>
    for Map<A>
where
    A: ArchiveWith<O> + DeserializeWith<<A as ArchiveWith<O>>::Archived, O, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<<A as ArchiveWith<O>>::Archived>,
        d: &mut D,
    ) -> Result<Vec<O>, D::Error> {
        field
            .iter()
            .map(|value| A::deserialize_with(value, d))
            .collect()
    }
}

// AsOwned

impl<'a, F: Archive + Clone> ArchiveWith<Cow<'a, F>> for AsOwned {
    type Archived = F::Archived;
    type Resolver = F::Resolver;

    #[inline]
    unsafe fn resolve_with(
        field: &Cow<'a, F>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        field.resolve(pos, resolver, out);
    }
}

impl<'a, F, S> SerializeWith<Cow<'a, F>, S> for AsOwned
where
    F: Serialize<S> + Clone,
    S: Fallible + ?Sized,
{
    #[inline]
    fn serialize_with(
        field: &Cow<'a, F>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        field.serialize(serializer)
    }
}

impl<T, D> DeserializeWith<T::Archived, T, D> for AsOwned
where
    T: Archive + Clone,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    #[inline]
    fn deserialize_with(
        field: &T::Archived,
        deserializer: &mut D,
    ) -> Result<T, D::Error> {
        field.deserialize(deserializer)
    }
}

impl<'a, T: Archive + Clone> ArchiveWith<Cow<'a, [T]>> for AsOwned {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    #[inline]
    unsafe fn resolve_with(
        field: &Cow<'a, [T]>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_slice(field, pos, resolver, out);
    }
}

impl<'a, T, S> SerializeWith<Cow<'a, [T]>, S> for AsOwned
where
    T: Serialize<S> + Clone,
    S: Fallible + Allocator + Writer + ?Sized,
{
    #[inline]
    fn serialize_with(
        field: &Cow<'a, [T]>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_slice(field, serializer)
    }
}

impl<'a, T, D> DeserializeWith<ArchivedVec<T::Archived>, Cow<'a, [T]>, D>
    for AsOwned
where
    T: Archive + Clone,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
    D::Error: Error,
{
    #[inline]
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<Cow<'a, [T]>, D::Error> {
        Ok(Cow::Owned(field.deserialize(deserializer)?))
    }
}

impl<'a> ArchiveWith<Cow<'a, str>> for AsOwned {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    unsafe fn resolve_with(
        field: &Cow<'a, str>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedString::resolve_from_str(field, pos, resolver, out);
    }
}

impl<'a, S> SerializeWith<Cow<'a, str>, S> for AsOwned
where
    S: Fallible + Writer + ?Sized,
{
    #[inline]
    fn serialize_with(
        field: &Cow<'a, str>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(field, serializer)
    }
}

impl<'a, D> DeserializeWith<ArchivedString, Cow<'a, str>, D> for AsOwned
where
    D: Fallible + ?Sized,
{
    #[inline]
    fn deserialize_with(
        field: &ArchivedString,
        deserializer: &mut D,
    ) -> Result<Cow<'a, str>, D::Error> {
        Ok(Cow::Owned(field.deserialize(deserializer)?))
    }
}

// TODO: Move this to the std module
#[cfg(feature = "std")]
const _: () = {
    use std::ffi::CStr;

    use crate::ffi::{ArchivedCString, CStringResolver};

    impl<'a> ArchiveWith<Cow<'a, CStr>> for AsOwned {
        type Archived = ArchivedCString;
        type Resolver = CStringResolver;

        #[inline]
        unsafe fn resolve_with(
            field: &Cow<'a, CStr>,
            pos: usize,
            resolver: Self::Resolver,
            out: *mut Self::Archived,
        ) {
            ArchivedCString::resolve_from_c_str(field, pos, resolver, out);
        }
    }

    impl<'a, S: Fallible + Writer + ?Sized> SerializeWith<Cow<'a, CStr>, S>
        for AsOwned
    {
        #[inline]
        fn serialize_with(
            field: &Cow<'a, CStr>,
            serializer: &mut S,
        ) -> Result<Self::Resolver, S::Error> {
            ArchivedCString::serialize_from_c_str(field, serializer)
        }
    }

    impl<'a, D> DeserializeWith<ArchivedCString, Cow<'a, CStr>, D> for AsOwned
    where
        D: Fallible + ?Sized,
        D::Error: Error,
    {
        #[inline]
        fn deserialize_with(
            field: &ArchivedCString,
            deserializer: &mut D,
        ) -> Result<Cow<'a, CStr>, D::Error> {
            Ok(Cow::Owned(field.deserialize(deserializer)?))
        }
    }
};

// AsVec

impl<K: Archive, V: Archive> ArchiveWith<BTreeMap<K, V>> for AsVec {
    type Archived = ArchivedVec<Entry<K::Archived, V::Archived>>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        field: &BTreeMap<K, V>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(field.len(), pos, resolver, out);
    }
}

impl<K, V, S> SerializeWith<BTreeMap<K, V>, S> for AsVec
where
    K: Serialize<S>,
    V: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &BTreeMap<K, V>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::serialize_from_iter(
            field.iter().map(|(key, value)| Entry { key, value }),
            serializer,
        )
    }
}

impl<K, V, D>
    DeserializeWith<
        ArchivedVec<Entry<K::Archived, V::Archived>>,
        BTreeMap<K, V>,
        D,
    > for AsVec
where
    K: Archive + Ord,
    V: Archive,
    K::Archived: Deserialize<K, D>,
    V::Archived: Deserialize<V, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<Entry<K::Archived, V::Archived>>,
        deserializer: &mut D,
    ) -> Result<BTreeMap<K, V>, D::Error> {
        let mut result = BTreeMap::new();
        for entry in field.iter() {
            result.insert(
                entry.key.deserialize(deserializer)?,
                entry.value.deserialize(deserializer)?,
            );
        }
        Ok(result)
    }
}

impl<T: Archive> ArchiveWith<BTreeSet<T>> for AsVec {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    unsafe fn resolve_with(
        field: &BTreeSet<T>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedVec::resolve_from_len(field.len(), pos, resolver, out);
    }
}

impl<T, S> SerializeWith<BTreeSet<T>, S> for AsVec
where
    T: Serialize<S>,
    S: Fallible + Allocator + Writer + ?Sized,
{
    fn serialize_with(
        field: &BTreeSet<T>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedVec::<T::Archived>::serialize_from_iter::<T, _, _>(
            field.iter(),
            serializer,
        )
    }
}

impl<T, D> DeserializeWith<ArchivedVec<T::Archived>, BTreeSet<T>, D> for AsVec
where
    T: Archive + Ord,
    T::Archived: Deserialize<T, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(
        field: &ArchivedVec<T::Archived>,
        deserializer: &mut D,
    ) -> Result<BTreeSet<T>, D::Error> {
        let mut result = BTreeSet::new();
        for key in field.iter() {
            result.insert(key.deserialize(deserializer)?);
        }
        Ok(result)
    }
}

// Niche

impl<T: ArchiveUnsized + ?Sized> ArchiveWith<Option<Box<T>>> for Niche
where
    ArchivedMetadata<T>: Default,
{
    type Archived = ArchivedOptionBox<T::Archived>;
    type Resolver = OptionBoxResolver;

    unsafe fn resolve_with(
        field: &Option<Box<T>>,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedOptionBox::resolve_from_option(
            field.as_deref(),
            pos,
            resolver,
            out,
        );
    }
}

impl<T, S> SerializeWith<Option<Box<T>>, S> for Niche
where
    T: SerializeUnsized<S> + ?Sized,
    S: Fallible + Writer + ?Sized,
    ArchivedMetadata<T>: Default,
{
    fn serialize_with(
        field: &Option<Box<T>>,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedOptionBox::serialize_from_option(field.as_deref(), serializer)
    }
}

impl<T, D> DeserializeWith<ArchivedOptionBox<T::Archived>, Option<Box<T>>, D>
    for Niche
where
    T: ArchiveUnsized + LayoutRaw + Pointee + ?Sized,
    T::Archived: DeserializeUnsized<T, D>,
    D: Fallible + ?Sized,
    D::Error: Error,
{
    fn deserialize_with(
        field: &ArchivedOptionBox<T::Archived>,
        deserializer: &mut D,
    ) -> Result<Option<Box<T>>, D::Error> {
        if let Some(value) = field.as_ref() {
            Ok(Some(value.deserialize(deserializer)?))
        } else {
            Ok(None)
        }
    }
}

// Cloned

impl<T: Archive> ArchiveWith<Arc<T>> for Cloned {
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    unsafe fn resolve_with(
        x: &Arc<T>,
        pos: usize,
        resolver: Self::Resolver,
        archived: *mut Self::Archived,
    ) {
        x.as_ref().resolve(pos, resolver, archived)
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> SerializeWith<Arc<T>, S>
    for Cloned
{
    fn serialize_with(
        x: &Arc<T>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        x.as_ref().serialize(s)
    }
}

impl<A: Deserialize<T, D>, T, D: Fallible + ?Sized>
    DeserializeWith<A, Arc<T>, D> for Cloned
{
    fn deserialize_with(x: &A, d: &mut D) -> Result<Arc<T>, D::Error> {
        Ok(Arc::new(A::deserialize(x, d)?))
    }
}

impl<T: Archive> ArchiveWith<Rc<T>> for Cloned {
    type Archived = T::Archived;
    type Resolver = T::Resolver;

    unsafe fn resolve_with(
        x: &Rc<T>,
        pos: usize,
        resolver: Self::Resolver,
        archived: *mut Self::Archived,
    ) {
        x.as_ref().resolve(pos, resolver, archived)
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> SerializeWith<Rc<T>, S> for Cloned {
    fn serialize_with(
        x: &Rc<T>,
        s: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        x.as_ref().serialize(s)
    }
}

impl<A: Deserialize<T, D>, T, D: Fallible + ?Sized> DeserializeWith<A, Rc<T>, D>
    for Cloned
{
    fn deserialize_with(x: &A, d: &mut D) -> Result<Rc<T>, D::Error> {
        Ok(Rc::new(A::deserialize(x, d)?))
    }
}
