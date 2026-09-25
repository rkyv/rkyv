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

unsafe impl LayoutRaw for CStr {
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

// SAFETY:
// - `deserialize_metadata` returns the byte length fixed in the `self`
//   reference, including its null terminator, so repeated calls are stable and
//   the caller provides an output `CStr` with that length.
// - The implementation copies all bytes of the valid input `CStr`, including
//   its null terminator, to `out` before returning `Ok(())`, so the output is a
//   valid `CStr`.
// - The implementation cannot return an error or unwind. It never reads from,
//   deallocates, or retains access to the output storage.
unsafe impl<D: Fallible + ?Sized> DeserializeUnsized<CStr, D> for CStr {
    unsafe fn deserialize_unsized(
        &self,
        _: &mut D,
        out: *mut CStr,
    ) -> Result<(), D::Error> {
        let slice = self.to_bytes_with_nul();
        // SAFETY: The caller has guaranteed that `out` is non-null, properly
        // aligned, and points to exclusively accessible storage valid for
        // writes of `self.len()` bytes. The output does not overlap memory
        // accessible through `self`. `self.as_ptr()` is valid for reads of
        // `self.len()` bytes. Note here that the length of the `CStr` contains
        // the null terminator.
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
