//! [`Archive`] implementations for std types.

pub mod chd;
#[cfg(feature = "validation")]
pub mod validation;

use crate::{
    core_impl::ArchivedSlice, Archive, ArchiveRef, Archived, Reference, ReferenceResolver, Resolve,
    Serialize, SerializeRef, Deserialize, DeserializeRef, Write,
};
use core::{
    borrow::Borrow,
    fmt,
    ops::{Deref, DerefMut, Index, IndexMut},
    pin::Pin,
    slice,
};
use std::alloc;

/// An archived [`String`].
///
/// Uses [`ArchivedStringSlice`](crate::core_impl::ArchivedStringSlice) under
/// the hood.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedString(Reference<str>);

impl ArchivedString {
    /// Extracts a string slice containing the entire `ArchivedString`.
    pub fn as_str(&self) -> &str {
        &**self
    }

    /// Converts an `ArchivedString` into a mutable string slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        &mut **self
    }

    /// Gets the value of this archived string as a pinned mutable reference.
    pub fn str_pin(self: Pin<&mut Self>) -> Pin<&mut str> {
        unsafe { self.map_unchecked_mut(|s| &mut **s) }
    }
}

impl Deref for ArchivedString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for ArchivedString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl Borrow<str> for ArchivedString {
    fn borrow(&self) -> &str {
        self.0.borrow()
    }
}

impl PartialEq<&str> for ArchivedString {
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(&**self, *other)
    }
}

impl PartialEq<ArchivedString> for &str {
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(&**other, *self)
    }
}

impl PartialEq<String> for ArchivedString {
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(&**self, &**other)
    }
}

impl PartialEq<ArchivedString> for String {
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(&**other, &**self)
    }
}

impl fmt::Display for ArchivedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

#[doc(hidden)]
pub struct StringResolver(ReferenceResolver<str>);

impl Resolve<String> for StringResolver {
    type Archived = ArchivedString;

    fn resolve(self, pos: usize, value: &String) -> Self::Archived {
        ArchivedString(self.0.resolve(pos, value.as_str()))
    }
}

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;
}

impl<W: Write + ?Sized> Serialize<W> for String {
    fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(StringResolver(self.as_str().serialize_ref(writer)?))
    }
}

impl Deserialize<String> for Archived<String> {
    fn deserialize(&self) -> String {
        self.as_str().to_string()
    }
}

/// An archived [`Box`].
///
/// This is a thin wrapper around the reference type for whatever type was
/// archived.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedBox<T>(T);

impl<T: DerefMut> ArchivedBox<T> {
    /// Gets the value of this archived box as a pinned mutable reference.
    pub fn get_pin(self: Pin<&mut Self>) -> Pin<&mut <T as Deref>::Target> {
        unsafe { self.map_unchecked_mut(|s| &mut **s) }
    }
}

impl<T: Deref> Deref for ArchivedBox<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: DerefMut> DerefMut for ArchivedBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

impl<T: Deref<Target = U>, U: PartialEq<V> + ?Sized, V: ?Sized> PartialEq<Box<V>>
    for ArchivedBox<T>
{
    fn eq(&self, other: &Box<V>) -> bool {
        self.deref().eq(other.deref())
    }
}

#[doc(hidden)]
pub struct BoxResolver<T>(T);

impl<T: ArchiveRef + ?Sized> Resolve<Box<T>> for BoxResolver<T::ReferenceResolver> {
    type Archived = ArchivedBox<T::Reference>;

    fn resolve(self, pos: usize, value: &Box<T>) -> Self::Archived {
        ArchivedBox(self.0.resolve(pos, value.as_ref()))
    }
}

impl<T: ArchiveRef + ?Sized> Archive for Box<T> {
    type Archived = ArchivedBox<Reference<T>>;
    type Resolver = BoxResolver<ReferenceResolver<T>>;
}

impl<T: SerializeRef<W> + ?Sized, W: Write + ?Sized> Serialize<W> for Box<T> {
    fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(BoxResolver(self.as_ref().serialize_ref(writer)?))
    }
}

impl<T: ArchiveRef + ?Sized> Deserialize<Box<T>> for Archived<Box<T>>
where
    Reference<T>: DeserializeRef<T>,
{
    fn deserialize(&self) -> Box<T> {
        unsafe { Box::from_raw(self.0.deserialize_ref(alloc::alloc)) }
    }
}

impl<T: Archive> ArchiveRef for [T] {
    type Archived = [T::Archived];
    type Reference = ArchivedSlice<T::Archived>;
    type ReferenceResolver = usize;
}

impl<T: Serialize<W>, W: Write + ?Sized> SerializeRef<W> for [T] {
    fn serialize_ref(&self, writer: &mut W) -> Result<Self::ReferenceResolver, W::Error> {
        if !self.is_empty() {
            let mut resolvers = Vec::with_capacity(self.len());
            for value in self {
                resolvers.push(value.serialize(writer)?);
            }
            let result = writer.align_for::<T::Archived>()?;
            unsafe {
                for (i, resolver) in resolvers.drain(..).enumerate() {
                    writer.resolve_aligned(&self[i], resolver)?;
                }
            }
            Ok(result)
        } else {
            Ok(0)
        }
    }
}

impl<T: Archive> DeserializeRef<[T]> for <[T] as ArchiveRef>::Reference
where
    T::Archived: Deserialize<T>,
{
    unsafe fn deserialize_ref(&self, alloc: unsafe fn(alloc::Layout) -> *mut u8) -> *mut [T] {
        let result = alloc(alloc::Layout::array::<T>(self.len()).unwrap()).cast::<T>();
        for i in 0..self.len() {
            result.add(i).write(self[i].deserialize());
        }
        slice::from_raw_parts_mut(result, self.len())
    }
}

/// An archived [`Vec`].
///
/// Uses [`ArchivedSlice`](crate::core_impl::ArchivedSlice) under the hood.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedVec<T>(T);

impl<T: DerefMut> ArchivedVec<T> {
    /// Gets the element at the given index ot this archived vec as a pinned
    /// mutable reference.
    pub fn index_pin<I>(self: Pin<&mut Self>, index: I) -> Pin<&mut <T::Target as Index<I>>::Output>
    where
        T::Target: IndexMut<I>,
    {
        unsafe { self.map_unchecked_mut(|s| &mut (**s)[index]) }
    }
}

impl<T: Deref> Deref for ArchivedVec<T> {
    type Target = T::Target;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl<T: DerefMut> DerefMut for ArchivedVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

#[doc(hidden)]
pub struct VecResolver<T>(T);

impl<T: Resolve<[U]>, U> Resolve<Vec<U>> for VecResolver<T> {
    type Archived = ArchivedVec<T::Archived>;

    fn resolve(self, pos: usize, value: &Vec<U>) -> Self::Archived {
        ArchivedVec(self.0.resolve(pos, value.deref()))
    }
}

impl<T: Archive> Archive for Vec<T> {
    type Archived = ArchivedVec<Reference<[T]>>;
    type Resolver = VecResolver<ReferenceResolver<[T]>>;
}

impl<T: Serialize<W>, W: Write + ?Sized> Serialize<W> for Vec<T> {
    fn serialize(&self, writer: &mut W) -> Result<Self::Resolver, W::Error> {
        Ok(VecResolver(self.as_slice().serialize_ref(writer)?))
    }
}

impl<T: Archive> Deserialize<Vec<T>> for Archived<Vec<T>>
where
    T::Archived: Deserialize<T>,
{
    fn deserialize(&self) -> Vec<T> {
        let mut result = Vec::with_capacity(self.len());
        for i in self.iter() {
            result.push(i.deserialize());
        }
        result
    }
}

impl<T: Deref<Target = [U]>, U: PartialEq<V>, V> PartialEq<Vec<V>> for ArchivedVec<T> {
    fn eq(&self, other: &Vec<V>) -> bool {
        self.deref().eq(other.deref())
    }
}
