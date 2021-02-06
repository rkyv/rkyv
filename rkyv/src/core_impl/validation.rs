//! Validation implementations for core types.

use crate::{
    core_impl::{
        range::{ArchivedRange, ArchivedRangeInclusive},
        ArchivedOption,
        ArchivedOptionTag,
        ArchivedOptionVariantSome,
        SliceMetadata,
    },
    offset_of,
};
use bytecheck::{CheckBytes, StructCheckError, Unreachable};
use core::{alloc::{Layout, LayoutErr}, fmt};
use std::error::Error;

#[derive(Debug)]
pub enum CheckSliceMetadataError {
    LayoutError(LayoutErr),
}

impl From<Unreachable> for CheckSliceMetadataError {
    fn from(_: Unreachable) -> Self {
        unreachable!()
    }
}

impl From<LayoutErr> for CheckSliceMetadataError {
    fn from(e: LayoutErr) -> Self {
        Self::LayoutError(e)
    }
}

impl fmt::Display for CheckSliceMetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckSliceMetadataError::LayoutError(e) => e.fmt(f),
        }
    }
}

impl Error for CheckSliceMetadataError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            CheckSliceMetadataError::LayoutError(e) => Some(e as &dyn Error),
        }
    }
}

impl<T, C: ?Sized> CheckBytes<C> for SliceMetadata<T> {
    type Error = CheckSliceMetadataError;

    unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        let len = u32::check_bytes(bytes.add(offset_of!(Self, len)).cast(), context)?;
        Layout::array::<T>(*len as usize)?;
        Ok(&*value)
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

impl<T: Error + 'static> Error for ArchivedOptionError<T> {
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

    unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        let tag = *u8::check_bytes(bytes, context)?;
        match tag {
            ArchivedOptionTag::TAG_NONE => (),
            ArchivedOptionTag::TAG_SOME => {
                T::check_bytes(
                    bytes.add(offset_of!(ArchivedOptionVariantSome<T>, 1)).cast(),
                    context,
                )
                .map_err(ArchivedOptionError::CheckBytes)?;
            }
            _ => return Err(ArchivedOptionError::InvalidTag(tag)),
        }
        Ok(&*value)
    }
}

impl<C: ?Sized, T: CheckBytes<C>> CheckBytes<C> for ArchivedRange<T> {
    type Error = StructCheckError;

    unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        T::check_bytes(bytes.add(offset_of!(ArchivedRange<T>, start)).cast(), context).map_err(|e| {
            StructCheckError {
                field_name: "start",
                inner: Box::new(e),
            }
        })?;
        T::check_bytes(bytes.add(offset_of!(ArchivedRange<T>, end)).cast(), context).map_err(|e| {
            StructCheckError {
                field_name: "end",
                inner: Box::new(e),
            }
        })?;
        Ok(&*value)
    }
}

impl<C: ?Sized, T: CheckBytes<C>> CheckBytes<C> for ArchivedRangeInclusive<T> {
    type Error = StructCheckError;

    unsafe fn check_bytes<'a>(value: *const Self, context: &mut C) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        T::check_bytes(
            bytes.add(offset_of!(ArchivedRangeInclusive<T>, start)).cast(),
            context,
        )
        .map_err(|e| StructCheckError {
            field_name: "start",
            inner: Box::new(e),
        })?;
        T::check_bytes(
            bytes.add(offset_of!(ArchivedRangeInclusive<T>, end)).cast(),
            context,
        )
        .map_err(|e| StructCheckError {
            field_name: "end",
            inner: Box::new(e),
        })?;
        Ok(&*value)
    }
}
