use core::{
    alloc::{Layout, LayoutError},
    ffi::{c_char, CStr},
    ptr,
};

use ptr_meta::Pointee;
use rancor::Fallible;

use crate::{
    primitive::ArchivedUsize,
    ser::Writer,
    traits::{ArchivePointee, LayoutRaw},
    ArchiveUnsized, ArchivedMetadata, DeserializeUnsized, Portable,
    SerializeUnsized,
};

// CStr

impl LayoutRaw for CStr {
    #[inline]
    fn layout_raw(
        metadata: <Self as Pointee>::Metadata,
    ) -> Result<Layout, LayoutError> {
        Layout::array::<c_char>(metadata)
    }
}

// SAFETY: `CStr` is a byte slice and so has a stable, well-defined layout that
// is the same on all targets
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

    fn deserialize_metadata(&self) -> <CStr as Pointee>::Metadata {
        ptr_meta::metadata(self)
    }
}
