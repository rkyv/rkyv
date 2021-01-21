//! Validation implementations for core types.

use crate::{
    core_impl::{
        range::{ArchivedRange, ArchivedRangeInclusive},
        ArchivedOption, ArchivedOptionTag, ArchivedOptionVariantSome, ArchivedRef, ArchivedSlice,
        ArchivedStringSlice,
    },
    offset_of,
    validation::{ArchiveBoundsContext, ArchiveBoundsError, CheckBytesRef},
    RelPtr,
};
use bytecheck::{CheckBytes, StructCheckError, Unreachable};
use core::{fmt, marker::PhantomData, mem, slice, str};
use std::error::Error;

impl<T, C: ?Sized> CheckBytes<C> for ArchivedRef<T> {
    type Error = Unreachable;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        RelPtr::check_bytes(bytes.add(offset_of!(Self, ptr)), context)?;
        PhantomData::<T>::check_bytes(bytes.add(offset_of!(Self, _phantom)), context)?;
        Ok(&*bytes.cast())
    }
}

impl<T: CheckBytes<C>, C: ArchiveBoundsContext + ?Sized> CheckBytesRef<C> for ArchivedRef<T> {
    type RefError = T::Error;
    type Target = T;

    fn check_ptr(&self, context: &mut C) -> Result<(*const u8, usize), ArchiveBoundsError> {
        unsafe {
            let len = mem::size_of::<T>();
            Ok((context.check_rel_ptr(&self.ptr, len, mem::align_of::<T>())?, len))
        }
    }

    unsafe fn check_ref_bytes<'a>(&'a self, bytes: *const u8, context: &mut C) -> Result<&'a Self::Target, Self::RefError> {
        T::check_bytes(bytes, context)
    }
}

impl<T: CheckBytes<C>, C: ?Sized> CheckBytes<C> for ArchivedSlice<T> {
    type Error = Unreachable;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        RelPtr::check_bytes(bytes.add(offset_of!(Self, ptr)), context)?;
        u32::check_bytes(bytes.add(offset_of!(Self, len)), context)?;
        Ok(&*bytes.cast())
    }
}

/// Errors that can occur while checking an [`ArchivedSlice`].
#[derive(Debug)]
pub enum SliceError<T> {
    /// An error occurred while checking the bytes of an element of the slice
    CheckBytes {
        /// The index of the element
        index: usize,
        /// The error that occurred
        inner: T,
    },
}

impl<T: fmt::Display> fmt::Display for SliceError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SliceError::CheckBytes { index, inner } => write!(f, "error at index {}: {}", index, inner),
        }
    }
}

impl<T: fmt::Debug + fmt::Display + Error + 'static> Error for SliceError<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            SliceError::CheckBytes { inner, .. } => Some(inner as &dyn Error),
        }
    }
}

impl<T: CheckBytes<C>, C: ArchiveBoundsContext + ?Sized> CheckBytesRef<C> for ArchivedSlice<T> {
    type RefError = SliceError<T::Error>;
    type Target = [T];

    fn check_ptr(&self, context: &mut C) -> Result<(*const u8, usize), ArchiveBoundsError> {
        unsafe {
            let len = self.len as usize * mem::size_of::<T>();
            Ok((context.check_rel_ptr(&self.ptr, len, mem::align_of::<T>())?, len))
        }
    }

    unsafe fn check_ref_bytes<'a>(&'a self, bytes: *const u8, context: &mut C) -> Result<&'a Self::Target, Self::RefError> {
        for i in 0..self.len as usize {
            T::check_bytes(bytes.add(i * core::mem::size_of::<T>()), context)
                .map_err(|e| SliceError::CheckBytes { index: i, inner: e })?;
        }
        Ok(slice::from_raw_parts(bytes.cast::<T>(), self.len as usize))
    }
}

impl<C: ?Sized> CheckBytes<C> for ArchivedStringSlice {
    type Error = Unreachable;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        ArchivedSlice::<u8>::check_bytes(bytes.add(offset_of!(Self, slice)), context)?;
        Ok(&*bytes.cast())
    }
}

/// Errors that can occur while checking an [`ArchivedStringSlice`].
#[derive(Debug)]
pub enum StringSliceError {
    /// The bytes of the string were invalid UTF-8
    InvalidUtf8(str::Utf8Error),
}

impl From<str::Utf8Error> for StringSliceError {
    fn from(e: str::Utf8Error) -> Self {
        Self::InvalidUtf8(e)
    }
}

impl fmt::Display for StringSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringSliceError::InvalidUtf8(e) => {
                write!(f, "archived string slice contained invalid UTF-8: {}", e)
            }
        }
    }
}

impl Error for StringSliceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            StringSliceError::InvalidUtf8(e) => Some(e as &dyn Error),
        }
    }
}

impl<C: ArchiveBoundsContext + ?Sized> CheckBytesRef<C> for ArchivedStringSlice {
    type RefError = StringSliceError;
    type Target = str;

    fn check_ptr(&self, context: &mut C) -> Result<(*const u8, usize), ArchiveBoundsError> {
        self.slice.check_ptr(context)
    }

    unsafe fn check_ref_bytes<'a>(&'a self, bytes: *const u8, context: &mut C) -> Result<&'a Self::Target, Self::RefError> {
        Ok(str::from_utf8(self.slice.check_ref_bytes(bytes, context).unwrap())?)
    }
}

/// Errors that can occur while checking an [`ArchivedOption`].
#[derive(Debug)]
pub enum ArchivedOptionError<T> {
    /// The option had an invalid tag
    InvalidTag(u8),
    /// An error occurred while checking the bytes of the target type
    CheckBytes(T),
}

impl<T: fmt::Display> fmt::Display for ArchivedOptionError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedOptionError::InvalidTag(tag) => write!(f, "archived option had invalid tag: {}", tag),
            ArchivedOptionError::CheckBytes(e) => write!(f, "archived option check error: {}", e),
        }
    }
}

impl<T: fmt::Debug + fmt::Display + Error + 'static> Error for ArchivedOptionError<T> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ArchivedOptionError::InvalidTag(_) => None,
            ArchivedOptionError::CheckBytes(e) => Some(e as &dyn Error),
        }
    }
}

impl<T> From<Unreachable> for ArchivedOptionError<T> {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl ArchivedOptionTag {
    const TAG_NONE: u8 = ArchivedOptionTag::None as u8;
    const TAG_SOME: u8 = ArchivedOptionTag::Some as u8;
}

impl<C: ?Sized, T: CheckBytes<C>> CheckBytes<C> for ArchivedOption<T> {
    type Error = ArchivedOptionError<T::Error>;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        let tag = *u8::check_bytes(bytes, context)?;
        match tag {
            ArchivedOptionTag::TAG_NONE => (),
            ArchivedOptionTag::TAG_SOME => {
                T::check_bytes(
                    bytes.add(offset_of!(ArchivedOptionVariantSome<T>, 1)),
                    context,
                )
                .map_err(ArchivedOptionError::CheckBytes)?;
            }
            _ => return Err(ArchivedOptionError::InvalidTag(tag)),
        }
        Ok(&*bytes.cast())
    }
}

impl<C: ?Sized, T: CheckBytes<C>> CheckBytes<C> for ArchivedRange<T> {
    type Error = StructCheckError;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        T::check_bytes(bytes.add(offset_of!(ArchivedRange<T>, start)), context).map_err(|e| {
            StructCheckError {
                field_name: "start",
                inner: Box::new(e),
            }
        })?;
        T::check_bytes(bytes.add(offset_of!(ArchivedRange<T>, end)), context).map_err(|e| {
            StructCheckError {
                field_name: "end",
                inner: Box::new(e),
            }
        })?;
        Ok(&*bytes.cast())
    }
}

impl<C: ?Sized, T: CheckBytes<C>> CheckBytes<C> for ArchivedRangeInclusive<T> {
    type Error = StructCheckError;

    unsafe fn check_bytes<'a>(bytes: *const u8, context: &mut C) -> Result<&'a Self, Self::Error> {
        T::check_bytes(
            bytes.add(offset_of!(ArchivedRangeInclusive<T>, start)),
            context,
        )
        .map_err(|e| StructCheckError {
            field_name: "start",
            inner: Box::new(e),
        })?;
        T::check_bytes(
            bytes.add(offset_of!(ArchivedRangeInclusive<T>, end)),
            context,
        )
        .map_err(|e| StructCheckError {
            field_name: "end",
            inner: Box::new(e),
        })?;
        Ok(&*bytes.cast())
    }
}
