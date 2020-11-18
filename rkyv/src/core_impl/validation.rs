//! Validation implementations for core types.

use crate::{
    core_impl::{
        ArchivedOption, ArchivedOptionTag, ArchivedOptionVariantSome, ArchivedRef, ArchivedSlice,
        ArchivedStringSlice,
    },
    offset_of,
    validation::{ArchiveContext, ArchiveMemoryError},
    RelPtr,
};
use bytecheck::{CheckBytes, Unreachable};
use core::{fmt, str};
use std::error::Error;

/// Errors that can occur while checking an [`ArchivedRef`].
#[derive(Debug)]
pub enum ArchivedRefError<T> {
    /// A memory error occurred
    MemoryError(ArchiveMemoryError),
    /// An error occurred while checking the bytes of the target type
    CheckBytes(T),
}

impl<T: fmt::Display> fmt::Display for ArchivedRefError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedRefError::MemoryError(e) => write!(f, "archived ref memory error: {}", e),
            ArchivedRefError::CheckBytes(e) => write!(f, "archived ref check error: {}", e),
        }
    }
}

impl<T: fmt::Debug + fmt::Display> Error for ArchivedRefError<T> {}

impl<T> From<ArchiveMemoryError> for ArchivedRefError<T> {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl<T> From<Unreachable> for ArchivedRefError<T> {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<T: CheckBytes<ArchiveContext>> CheckBytes<ArchiveContext> for ArchivedRef<T> {
    type Error = ArchivedRefError<T::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::check_bytes(bytes, context)?;
        let target = context.claim::<T>(bytes, rel_ptr.offset(), 1)?;
        T::check_bytes(target, context).map_err(ArchivedRefError::CheckBytes)?;
        Ok(&*bytes.cast())
    }
}

/// Errors that can occur while checking an [`ArchivedSlice`].
#[derive(Debug)]
pub enum ArchivedSliceError<T> {
    /// A memory error occurred
    MemoryError(ArchiveMemoryError),
    /// An error occurred while checking the bytes of an item of the target type
    CheckBytes(usize, T),
}

impl<T: fmt::Display> fmt::Display for ArchivedSliceError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedSliceError::MemoryError(e) => write!(f, "archived slice memory error: {}", e),
            ArchivedSliceError::CheckBytes(index, e) => {
                write!(f, "archived slice index {} check error: {}", index, e)
            }
        }
    }
}

impl<T: fmt::Debug + fmt::Display> Error for ArchivedSliceError<T> {}

impl<T> From<ArchiveMemoryError> for ArchivedSliceError<T> {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl<T> From<Unreachable> for ArchivedSliceError<T> {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl<T: CheckBytes<ArchiveContext>> CheckBytes<ArchiveContext> for ArchivedSlice<T> {
    type Error = ArchivedSliceError<T::Error>;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        let rel_ptr = RelPtr::check_bytes(bytes.add(offset_of!(Self, ptr)), context)?;
        let len = *u32::check_bytes(bytes.add(offset_of!(Self, len)), context)? as usize;
        let target = context.claim::<T>(bytes, rel_ptr.offset(), len)?;
        for i in 0..len {
            T::check_bytes(target.add(i * core::mem::size_of::<T>()), context)
                .map_err(|e| ArchivedSliceError::CheckBytes(i, e))?;
        }
        Ok(&*bytes.cast())
    }
}

/// Errors that can occur while checking an [`ArchivedStringSlice`].
#[derive(Debug)]
pub enum ArchivedStringSliceError {
    /// A memory error occurred
    MemoryError(ArchiveMemoryError),
    /// The bytes of the string were invalid UTF-8
    InvalidUtf8(str::Utf8Error),
}

impl fmt::Display for ArchivedStringSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedStringSliceError::MemoryError(e) => {
                write!(f, "archived string slice memory error: {}", e)
            }
            ArchivedStringSliceError::InvalidUtf8(e) => {
                write!(f, "archived string slice contained invalid UTF-8: {}", e)
            }
        }
    }
}

impl Error for ArchivedStringSliceError {}

impl From<ArchiveMemoryError> for ArchivedStringSliceError {
    fn from(e: ArchiveMemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl From<Unreachable> for ArchivedStringSliceError {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl CheckBytes<ArchiveContext> for ArchivedStringSlice {
    type Error = ArchivedStringSliceError;

    unsafe fn check_bytes<'a>(
        bytes: *const u8,
        context: &mut ArchiveContext,
    ) -> Result<&'a Self, Self::Error> {
        let slice = ArchivedSlice::<u8>::check_bytes(bytes, context).map_err(|e| match e {
            ArchivedSliceError::MemoryError(e) => e,
            ArchivedSliceError::CheckBytes(..) => unreachable!(),
        })?;
        str::from_utf8(&**slice).map_err(ArchivedStringSliceError::InvalidUtf8)?;
        Ok(&*bytes.cast())
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
            ArchivedOptionError::InvalidTag(tag) => {
                write!(f, "archived option had invalid tag: {}", tag)
            }
            ArchivedOptionError::CheckBytes(e) => write!(f, "archived option check error: {}", e),
        }
    }
}

impl<T: fmt::Debug + fmt::Display> Error for ArchivedOptionError<T> {}

impl<T> From<Unreachable> for ArchivedOptionError<T> {
    fn from(_: Unreachable) -> Self {
        unreachable!();
    }
}

impl ArchivedOptionTag {
    const TAG_NONE: u8 = ArchivedOptionTag::None as u8;
    const TAG_SOME: u8 = ArchivedOptionTag::Some as u8;
}

impl<C, T: CheckBytes<C>> CheckBytes<C> for ArchivedOption<T> {
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
