use core::ffi::CStr;

use rancor::{Fallible, ResultExt, Source};

use crate::{
    alloc::{alloc::alloc, boxed::Box, ffi::CString},
    ffi::{ArchivedCString, CStringResolver},
    ser::Writer,
    traits::LayoutRaw,
    Archive, Deserialize, DeserializeUnsized, Place, Serialize,
};

// CString

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
        let metadata = self.as_c_str().deserialize_metadata();
        let layout = <CStr as LayoutRaw>::layout_raw(metadata).into_error()?;
        let data_address = if layout.size() > 0 {
            unsafe { alloc(layout) }
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

#[cfg(test)]
mod tests {
    use crate::{
        alloc::{ffi::CString, string::String},
        api::test::roundtrip,
    };

    #[test]
    fn roundtrip_c_string() {
        let value = unsafe {
            CString::from_vec_unchecked(
                String::from("hello world").into_bytes(),
            )
        };
        roundtrip(&value);
    }
}
