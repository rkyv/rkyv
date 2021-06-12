//! [`Archive`] implementations for std types.

pub mod chd;
pub mod net;
pub mod shared;
#[cfg(feature = "validation")]
pub mod validation;

use crate::{
    ser::Serializer, Archive, ArchivePointee, ArchiveUnsized, Archived, ArchivedMetadata,
    Deserialize, DeserializeUnsized, Fallible, FixedUsize, MetadataResolver, RelPtr, Serialize,
    SerializeUnsized,
};
use core::{
    alloc::Layout,
    borrow::Borrow,
    cmp, fmt, hash,
    mem::MaybeUninit,
    ops::{Deref, DerefMut, Index, IndexMut},
    pin::Pin,
    ptr,
};
use ptr_meta::Pointee;
use std::ffi::{CStr, CString};

impl ArchiveUnsized for CStr {
    type Archived = CStr;

    type MetadataResolver = ();

    #[inline]
    unsafe fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        out: &mut MaybeUninit<ArchivedMetadata<Self>>,
    ) {
        out.as_mut_ptr()
            .write(to_archived!(ptr_meta::metadata(self) as FixedUsize))
    }
}

impl ArchivePointee for CStr {
    type ArchivedMetadata = Archived<usize>;

    #[inline]
    fn pointer_metadata(archived: &Self::ArchivedMetadata) -> <Self as Pointee>::Metadata {
        <[u8]>::pointer_metadata(archived)
    }
}

impl<S: Serializer + ?Sized> SerializeUnsized<S> for CStr {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        let result = serializer.pos();
        serializer.write(self.to_bytes_with_nul())?;
        Ok(result)
    }

    #[inline]
    fn serialize_metadata(&self, _: &mut S) -> Result<Self::MetadataResolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> DeserializeUnsized<CStr, D> for <CStr as ArchiveUnsized>::Archived {
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), D::Error> {
        let slice = self.to_bytes_with_nul();
        let bytes = alloc(Layout::array::<u8>(slice.len()).unwrap());
        ptr::copy_nonoverlapping(slice.as_ptr(), bytes, slice.len());
        Ok(bytes.cast())
    }

    #[inline]
    fn deserialize_metadata(&self, _: &mut D) -> Result<<CStr as Pointee>::Metadata, D::Error> {
        Ok(ptr_meta::metadata(self))
    }
}

/// An archived [`String`].
///
/// Uses a [`RelPtr`] to a `str` under the hood.
#[derive(Debug)]
#[repr(transparent)]
pub struct ArchivedString(RelPtr<str>);

impl ArchivedString {
    /// Extracts a string slice containing the entire `ArchivedString`.
    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { &*self.0.as_ptr() }
    }

    /// Converts an `ArchivedString` into a mutable string slice.
    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { &mut *self.0.as_mut_ptr() }
    }

    /// Gets the value of this archived string as a pinned mutable reference.
    #[inline]
    pub fn str_pin(self: Pin<&mut Self>) -> Pin<&mut str> {
        unsafe { self.map_unchecked_mut(|s| s.as_mut_str()) }
    }

    /// Resolves the archived string from a given `str`.
    ///
    /// # Safety
    ///
    /// - `pos` must be the position of `out` within the archive
    /// - `resolver` must be the result of serializing `value`
    #[inline]
    pub unsafe fn resolve_from_str(
        value: &str,
        pos: usize,
        resolver: StringResolver,
        out: &mut MaybeUninit<Self>,
    ) {
        let (fp, fo) = out_field!(out.0);
        #[allow(clippy::unit_arg)]
        value.resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }

    /// Serializes the archived string from a given `str`.
    #[inline]
    pub fn serialize_from_str<S: Fallible + ?Sized>(
        value: &str,
        serializer: &mut S,
    ) -> Result<StringResolver, S::Error>
    where
        str: SerializeUnsized<S>,
    {
        Ok(StringResolver {
            pos: value.serialize_unsized(serializer)?,
            metadata_resolver: value.serialize_metadata(serializer)?,
        })
    }
}

impl cmp::Eq for ArchivedString {}

impl hash::Hash for ArchivedString {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl cmp::Ord for ArchivedString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl cmp::PartialEq for ArchivedString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl cmp::PartialOrd for ArchivedString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl AsRef<str> for ArchivedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsMut<str> for ArchivedString {
    fn as_mut(&mut self) -> &mut str {
        self.as_mut_str()
    }
}

impl Deref for ArchivedString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for ArchivedString {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_str()
    }
}

impl Borrow<str> for ArchivedString {
    #[inline]
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq<&str> for ArchivedString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        PartialEq::eq(self.as_str(), *other)
    }
}

impl PartialEq<ArchivedString> for &str {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), *self)
    }
}

impl PartialEq<String> for ArchivedString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        PartialEq::eq(self.as_str(), other.as_str())
    }
}

impl PartialEq<ArchivedString> for String {
    #[inline]
    fn eq(&self, other: &ArchivedString) -> bool {
        PartialEq::eq(other.as_str(), self.as_str())
    }
}

impl fmt::Display for ArchivedString {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), f)
    }
}

/// The resolver for `String`.
pub struct StringResolver {
    pos: usize,
    metadata_resolver: MetadataResolver<str>,
}

impl Archive for String {
    type Archived = ArchivedString;
    type Resolver = StringResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: StringResolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedString::resolve_from_str(self.as_str(), pos, resolver, out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for String
where
    str: SerializeUnsized<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedString::serialize_from_str(self.as_str(), serializer)
    }
}

impl<D: Fallible + ?Sized> Deserialize<String, D> for Archived<String>
where
    str: DeserializeUnsized<str, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<String, D::Error> {
        unsafe {
            let data_address = self
                .as_str()
                .deserialize_unsized(deserializer, |layout| alloc::alloc::alloc(layout))?;
            let metadata = self.0.metadata().deserialize(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::<str>::from_raw(ptr).into())
        }
    }
}

/// An archived [`CString`].
///
/// Uses a [`RelPtr`] to a `CStr` under the hood.
#[derive(Debug)]
#[repr(transparent)]
pub struct ArchivedCString(RelPtr<CStr>);

impl ArchivedCString {
    /// Returns the contents of this CString as a slice of bytes.
    ///
    /// The returned slice does **not** contain the trailing nul terminator, and it is guaranteed to
    /// not have any interior nul bytes. If you need the nul terminator, use
    /// [`as_bytes_with_nul`][ArchivedCString::as_bytes_with_nul()] instead.
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.as_c_str().to_bytes()
    }

    /// Equivalent to [`as_bytes`][ArchivedCString::as_bytes()] except that the returned slice
    /// includes the trailing nul terminator.
    #[inline]
    pub fn as_bytes_with_nul(&self) -> &[u8] {
        &self.as_c_str().to_bytes_with_nul()
    }

    /// Extracts a `CStr` slice containing the entire string.
    #[inline]
    pub fn as_c_str(&self) -> &CStr {
        unsafe { &*self.0.as_ptr() }
    }

    /// Extracts a mutable `CStr` slice containing the entire string.
    #[inline]
    pub fn as_mut_c_str(&mut self) -> &mut CStr {
        unsafe { &mut *self.0.as_mut_ptr() }
    }
}

impl cmp::Eq for ArchivedCString {}

impl hash::Hash for ArchivedCString {
    #[inline]
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        self.as_bytes_with_nul().hash(state);
    }
}

impl cmp::Ord for ArchivedCString {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_bytes().cmp(other.as_bytes())
    }
}

impl cmp::PartialEq for ArchivedCString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl cmp::PartialOrd for ArchivedCString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_bytes().partial_cmp(other.as_bytes())
    }
}

impl AsRef<CStr> for ArchivedCString {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl AsMut<CStr> for ArchivedCString {
    fn as_mut(&mut self) -> &mut CStr {
        self.as_mut_c_str()
    }
}

impl Deref for ArchivedCString {
    type Target = CStr;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_c_str()
    }
}

impl DerefMut for ArchivedCString {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_c_str()
    }
}

impl Borrow<CStr> for ArchivedCString {
    #[inline]
    fn borrow(&self) -> &CStr {
        self.as_c_str()
    }
}

impl PartialEq<&CStr> for ArchivedCString {
    #[inline]
    fn eq(&self, other: &&CStr) -> bool {
        PartialEq::eq(self.as_c_str(), other)
    }
}

impl PartialEq<ArchivedCString> for &CStr {
    #[inline]
    fn eq(&self, other: &ArchivedCString) -> bool {
        PartialEq::eq(other.as_c_str(), self)
    }
}

impl PartialEq<CString> for ArchivedCString {
    #[inline]
    fn eq(&self, other: &CString) -> bool {
        PartialEq::eq(self.as_c_str(), other.as_c_str())
    }
}

impl PartialEq<ArchivedCString> for CString {
    #[inline]
    fn eq(&self, other: &ArchivedCString) -> bool {
        PartialEq::eq(other.as_c_str(), self.as_c_str())
    }
}

/// The resolver for `CString`.
pub struct CStringResolver {
    pos: usize,
    metadata_resolver: MetadataResolver<CStr>,
}

impl Archive for CString {
    type Archived = ArchivedCString;
    type Resolver = CStringResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        let (fp, fo) = out_field!(out.0);
        #[allow(clippy::unit_arg)]
        self.as_c_str().resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for CString
where
    CStr: SerializeUnsized<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(CStringResolver {
            pos: self.as_c_str().serialize_unsized(serializer)?,
            metadata_resolver: self.as_c_str().serialize_metadata(serializer)?,
        })
    }
}

impl<D: Fallible + ?Sized> Deserialize<CString, D> for Archived<CString>
where
    CStr: DeserializeUnsized<CStr, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<CString, D::Error> {
        unsafe {
            let data_address = self
                .as_c_str()
                .deserialize_unsized(deserializer, |layout| alloc::alloc::alloc(layout))?;
            let metadata = self.0.metadata().deserialize(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::<CStr>::from_raw(ptr).into())
        }
    }
}

/// An archived [`Box`].
///
/// This is a thin wrapper around a [`RelPtr`] to the archived type.
#[repr(transparent)]
pub struct ArchivedBox<T: ArchivePointee + ?Sized>(RelPtr<T>);

impl<T: ArchivePointee + ?Sized> ArchivedBox<T> {
    /// Gets the value of this archived box as a pinned mutable reference.
    #[inline]
    pub fn get_pin(self: Pin<&mut Self>) -> Pin<&mut T> {
        unsafe { self.map_unchecked_mut(|s| s.deref_mut()) }
    }

    #[inline]
    pub unsafe fn resolve_from_ref<U: ArchiveUnsized<Archived = T> + ?Sized>(
        value: &U,
        pos: usize,
        resolver: BoxResolver<U::MetadataResolver>,
        out: &mut MaybeUninit<Self>,
    ) {
        let (fp, fo) = out_field!(out.0);
        value.resolve_unsized(pos + fp, resolver.pos, resolver.metadata_resolver, fo);
    }

    #[inline]
    pub fn serialize_from_ref<U: SerializeUnsized<S, Archived = T> + ?Sized, S: Fallible + ?Sized>(
        value: &U,
        serializer: &mut S,
    ) -> Result<BoxResolver<U::MetadataResolver>, S::Error> {
        Ok(BoxResolver {
            pos: value.serialize_unsized(serializer)?,
            metadata_resolver: value.serialize_metadata(serializer)?,
        })
    } 
}

impl<T: ArchivePointee + ?Sized> AsRef<T> for ArchivedBox<T> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T: ArchivePointee + ?Sized> AsMut<T> for ArchivedBox<T> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0.as_mut_ptr() }
    }
}

impl<T: ArchivePointee + ?Sized> fmt::Debug for ArchivedBox<T>
where
    T::ArchivedMetadata: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ArchivedBox").field(&self.0).finish()
    }
}

impl<T: ArchivePointee + ?Sized> Deref for ArchivedBox<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T: ArchivePointee + ?Sized> DerefMut for ArchivedBox<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T: ArchivePointee + PartialEq<U> + ?Sized, U: ?Sized> PartialEq<Box<U>> for ArchivedBox<T> {
    #[inline]
    fn eq(&self, other: &Box<U>) -> bool {
        self.deref().eq(other.deref())
    }
}

/// The resolver for `Box`.
pub struct BoxResolver<T> {
    pos: usize,
    metadata_resolver: T,
}

impl<T: ArchiveUnsized + ?Sized> Archive for Box<T> {
    type Archived = ArchivedBox<T::Archived>;
    type Resolver = BoxResolver<T::MetadataResolver>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        ArchivedBox::resolve_from_ref(self.as_ref(), pos, resolver, out);
    }
}

impl<T: SerializeUnsized<S> + ?Sized, S: Fallible + ?Sized> Serialize<S> for Box<T> {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        ArchivedBox::serialize_from_ref(self.as_ref(), serializer)
    }
}

impl<T: ArchiveUnsized + ?Sized, D: Fallible + ?Sized> Deserialize<Box<T>, D> for Archived<Box<T>>
where
    T::Archived: DeserializeUnsized<T, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Box<T>, D::Error> {
        unsafe {
            let data_address = self
                .deref()
                .deserialize_unsized(deserializer, |layout| alloc::alloc::alloc(layout))?;
            let metadata = self.deref().deserialize_metadata(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::from_raw(ptr))
        }
    }
}

/// An archived [`Vec`].
///
/// Uses a [`RelPtr`] to a `T` slice under the hood.
#[derive(Debug)]
#[repr(transparent)]
pub struct ArchivedVec<T>(RelPtr<[T]>);

impl<T> ArchivedVec<T> {
    /// Gets the elements of the archived vec as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[T] {
        self.deref()
    }

    /// Gets the elements of the archived vec as a mutable slice.
    #[inline]
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        self.deref_mut()
    }

    /// Gets the element at the given index ot this archived vec as a pinned mutable reference.
    #[inline]
    pub fn index_pin<I>(self: Pin<&mut Self>, index: I) -> Pin<&mut <[T] as Index<I>>::Output>
    where
        [T]: IndexMut<I>,
    {
        unsafe { self.map_unchecked_mut(|s| &mut s.deref_mut()[index]) }
    }
}

impl<T> Deref for ArchivedVec<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> DerefMut for ArchivedVec<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_mut_ptr() }
    }
}

/// The resolver for `Vec`.
pub struct VecResolver {
    pos: usize,
}

impl<T: Archive> Archive for Vec<T> {
    type Archived = ArchivedVec<T::Archived>;
    type Resolver = VecResolver;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        let (fp, fo) = out_field!(out.0);
        self.as_slice().resolve_unsized(pos + fp, resolver.pos, (), fo);
    }
}

impl<T: Serialize<S>, S: Fallible + ?Sized> Serialize<S> for Vec<T>
where
    [T]: SerializeUnsized<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        self.as_slice().serialize_metadata(serializer)?;
        Ok(VecResolver {
            pos: self.as_slice().serialize_unsized(serializer)?,
        })
    }
}

impl<T: Archive, D: Fallible + ?Sized> Deserialize<Vec<T>, D> for Archived<Vec<T>>
where
    [T::Archived]: DeserializeUnsized<[T], D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<Vec<T>, D::Error> {
        unsafe {
            let data_address = self
                .as_slice()
                .deserialize_unsized(deserializer, |layout| alloc::alloc::alloc(layout))?;
            let metadata = self.as_slice().deserialize_metadata(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::<[T]>::from_raw(ptr).into())
        }
    }
}

impl<T: PartialEq<U>, U> PartialEq<Vec<U>> for ArchivedVec<T> {
    #[inline]
    fn eq(&self, other: &Vec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}

impl<T: PartialEq<U>, U> PartialEq<ArchivedVec<U>> for Vec<T> {
    #[inline]
    fn eq(&self, other: &ArchivedVec<U>) -> bool {
        self.as_slice().eq(other.as_slice())
    }
}
