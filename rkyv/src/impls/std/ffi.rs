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
    Archive, ArchivePointee, ArchiveUnsized, ArchivedMetadata, Deserialize,
    DeserializeUnsized, LayoutRaw, Place, Portable, Serialize,
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
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, S::Error> {
        let result = serializer.pos();
        serializer.write(self.to_bytes_with_nul())?;
        Ok(result)
    }
}

impl<D: Fallible + ?Sized> DeserializeUnsized<CStr, D> for CStr {
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        out: *mut CStr,
    ) -> Result<(), D::Error> {
        let slice = self.to_bytes_with_nul();
        // SAFETY: The caller has guaranteed that `out` is non-null, properly
        // aligned, valid for writes, and points to memory allocated according
        // to the layout for the metadata returned from `deserialize_metadata`.
        // Therefore, `out` points to at least `self.len()` bytes.
        // `self.as_ptr()` is valid for reads and points to the bytes of `self`
        // which are also at least `self.len()` bytes. Note here that the length
        // of the `CStr` contains the null terminator.
        unsafe {
            ptr::copy_nonoverlapping(
                slice.as_ptr(),
                out.cast::<u8>(),
                slice.len(),
            );
        }
        Ok(())
    }

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
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedCString::resolve_from_c_str(self.as_c_str(), resolver, out);
    }
}

impl<S: Fallible + Writer + ?Sized> Serialize<S> for CString {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        ArchivedCString::serialize_from_c_str(self.as_c_str(), serializer)
    }
}

impl<D> Deserialize<CString, D> for ArchivedCString
where
    D: Fallible + ?Sized,
    D::Error: Source,
    CStr: DeserializeUnsized<CStr, D>,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<CString, D::Error> {
        let metadata = self.as_c_str().deserialize_metadata(deserializer)?;
        let layout = <CStr as LayoutRaw>::layout_raw(metadata).into_error()?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc::alloc(layout) }
        } else {
            crate::polyfill::dangling(&layout).as_ptr()
        };
        let out = ptr_meta::from_raw_parts_mut(data_address.cast(), metadata);
        unsafe {
            self.as_c_str().deserialize_unsized(deserializer, out)?;
        }
        let boxed = unsafe { Box::<CStr>::from_raw(out) };
        Ok(CString::from(boxed))
    }
}
