use crate::{
    ffi::{ArchivedCString, CStringResolver},
    primitive::ArchivedUsize,
    ser::Serializer,
    Archive, ArchivePointee, ArchiveUnsized, Archived, ArchivedMetadata,
    Deserialize, DeserializeUnsized, Serialize, SerializeUnsized,
};
use core::{alloc::Layout, ptr};
use ptr_meta::Pointee;
use std::alloc;
use std::ffi::{CStr, CString};

// CStr

impl ArchiveUnsized for CStr {
    type Archived = CStr;

    type MetadataResolver = ();

    #[inline]
    unsafe fn resolve_metadata(
        &self,
        _: usize,
        _: Self::MetadataResolver,
        out: *mut ArchivedMetadata<Self>,
    ) {
        out.write(ArchivedUsize::from_native(ptr_meta::metadata(self) as _))
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

impl<S: Serializer<E> + ?Sized, E> SerializeUnsized<S, E> for CStr {
    #[inline]
    fn serialize_unsized(&self, serializer: &mut S) -> Result<usize, E> {
        let result = serializer.pos();
        serializer.write(self.to_bytes_with_nul())?;
        Ok(result)
    }

    #[inline]
    fn serialize_metadata(
        &self,
        _: &mut S,
    ) -> Result<Self::MetadataResolver, E> {
        Ok(())
    }
}

impl<D: ?Sized, E> DeserializeUnsized<CStr, D, E> for <CStr as ArchiveUnsized>::Archived {
    #[inline]
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        mut alloc: impl FnMut(Layout) -> *mut u8,
    ) -> Result<*mut (), E> {
        let slice = self.to_bytes_with_nul();
        let bytes = alloc(Layout::array::<u8>(slice.len()).unwrap());
        assert!(!bytes.is_null());
        ptr::copy_nonoverlapping(slice.as_ptr(), bytes, slice.len());
        Ok(bytes.cast())
    }

    #[inline]
    fn deserialize_metadata(
        &self,
        _: &mut D,
    ) -> Result<<CStr as Pointee>::Metadata, E> {
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

impl<S: Serializer<E> + ?Sized, E> Serialize<S, E> for CString {
    #[inline]
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, E> {
        ArchivedCString::serialize_from_c_str(self.as_c_str(), serializer)
    }
}

impl<D: ?Sized, E> Deserialize<CString, D, E> for Archived<CString>
where
    CStr: DeserializeUnsized<CStr, D, E>,
{
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<CString, E> {
        unsafe {
            let data_address = self
                .as_c_str()
                .deserialize_unsized(deserializer, |layout| {
                    alloc::alloc(layout)
                })?;
            let metadata =
                self.as_c_str().deserialize_metadata(deserializer)?;
            let ptr = ptr_meta::from_raw_parts_mut(data_address, metadata);
            Ok(Box::<CStr>::from_raw(ptr).into())
        }
    }
}
