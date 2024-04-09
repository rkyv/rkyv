use core::{
    alloc::{Layout, LayoutError},
    ptr,
};
use std::{
    alloc,
    ffi::{CStr, CString},
};

use ptr_meta::Pointee;
use rancor::{Fallible, ResultExt, Source};

use crate::{
    ffi::{ArchivedCString, CStringResolver},
    primitive::ArchivedUsize,
    ser::Writer,
    Archive, ArchivePointee, ArchiveUnsized, Archived, ArchivedMetadata,
    Deserialize, DeserializeUnsized, LayoutRaw, Portable, Serialize,
    SerializeUnsized,
};

// CStr

impl LayoutRaw for CStr {
    #[inline]
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        Layout::array::<::std::os::raw::c_char>(metadata)
    }
}

unsafe impl Portable for CStr {}

impl ArchiveUnsized for CStr {
    type Archived = CStr;

    #[inline]
    fn archived_metadata(&self) -> ArchivedMetadata<Self> {
        ArchivedUsize::from_native(ptr_meta::metadata(self) as _)
    }
}

impl ArchivePointee for CStr {
    type ArchivedMetadata = ArchivedUsize;

    #[inline]
    fn pointer_metadata(
        archived: &Self::ArchivedMetadata,
    ) -> <Self as Pointee>::Metadata {
        <[u8]>::pointer_metadata(archived)
    }
}

impl<S: Fallible + Writer + ?Sized> SerializeUnsized<S> for CStr {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        let result = serializer.pos();
        serializer.write(self.to_bytes_with_nul())?;
        Ok(result)
    }
}

impl<D: Fallible + ?Sized> DeserializeUnsized<CStr, D>
    for <CStr as ArchiveUnsized>::Archived
{
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        out: *mut CStr,
    ) -> Result<(), D::Error> {
        let slice = self.to_bytes_with_nul();
        ptr::copy_nonoverlapping(slice.as_ptr(), out.cast::<u8>(), slice.len());
        Ok(())
    }

    #[inline]
    fn deserialize_metadata(
        &self,
        _: &mut D,
    ) -> Result<<CStr as Pointee>::Metadata, D::Error> {
        Ok(ptr_meta::metadata(self))
    }
}

// CString

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

impl Archive for CString {
    type Archived = ArchivedCString;
    type Resolver = CStringResolver;

    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        ArchivedCString::resolve_from_c_str(
            self.as_c_str(),
            pos,
            resolver,
            out,
        );
    }
}

impl<S: Fallible + Writer + ?Sized> Serialize<S> for CString {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedCString::serialize_from_c_str(self.as_c_str(), serializer)
    }
}

impl<D> Deserialize<CString, D> for Archived<CString>
where
    D: Fallible + ?Sized,
    D::Error: Source,
    CStr: DeserializeUnsized<CStr, D>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<CString, D::Error> {
        let metadata = self.as_c_str().deserialize_metadata(deserializer)?;
        let layout = <CStr as LayoutRaw>::layout_raw(metadata).into_error()?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc::alloc(layout) }
        } else {
            layout.align() as *mut u8
        };
        let out = ptr_meta::from_raw_parts_mut(data_address.cast(), metadata);
        unsafe {
            self.as_c_str().deserialize_unsized(deserializer, out)?;
        }
        let boxed = unsafe { Box::<CStr>::from_raw(out) };
        Ok(CString::from(boxed))
    }
}
