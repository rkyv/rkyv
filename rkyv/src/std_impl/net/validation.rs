//! Validation implementations for network types.

use crate::{
    offset_of,
    std_impl::net::{
        ArchivedIpAddr, ArchivedIpAddrTag, ArchivedIpAddrVariantV4, ArchivedIpAddrVariantV6,
        ArchivedIpv4Addr, ArchivedIpv6Addr, ArchivedSocketAddr, ArchivedSocketAddrTag,
        ArchivedSocketAddrV4, ArchivedSocketAddrV6, ArchivedSocketAddrVariantV4,
        ArchivedSocketAddrVariantV6,
    },
};
use bytecheck::{CheckBytes, Unreachable};
use core::fmt;
use std::error::Error;

/// Errors that can occur while checking an [`ArchivedIpAddr`].
#[derive(Debug)]
pub enum ArchivedIpAddrError {
    /// The option had an invalid tag
    InvalidTag(u8),
    InvalidIpv4Addr(<ArchivedIpv4Addr as CheckBytes<()>>::Error),
    InvalidIpv6Addr(<ArchivedIpv6Addr as CheckBytes<()>>::Error),
}

impl fmt::Display for ArchivedIpAddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedIpAddrError::InvalidTag(tag) => {
                write!(f, "archived IP address had invalid tag: {}", tag)
            }
            ArchivedIpAddrError::InvalidIpv4Addr(e) => {
                write!(f, "archived IPv4 address check error: {}", e)
            }
            ArchivedIpAddrError::InvalidIpv6Addr(e) => {
                write!(f, "archived IPv6 address check error: {}", e)
            }
        }
    }
}

impl Error for ArchivedIpAddrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ArchivedIpAddrError::InvalidTag(_) => None,
            ArchivedIpAddrError::InvalidIpv4Addr(e) => Some(e as &dyn Error),
            ArchivedIpAddrError::InvalidIpv6Addr(e) => Some(e as &dyn Error),
        }
    }
}

impl From<Unreachable> for ArchivedIpAddrError {
    fn from(_: Unreachable) -> Self {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

impl ArchivedIpAddrTag {
    const TAG_V4: u8 = ArchivedIpAddrTag::V4 as u8;
    const TAG_V6: u8 = ArchivedIpAddrTag::V6 as u8;
}

impl<C: ?Sized> CheckBytes<C> for ArchivedIpAddr {
    type Error = ArchivedIpAddrError;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        let tag = *u8::check_bytes(bytes, context)?;
        match tag {
            ArchivedIpAddrTag::TAG_V4 => {
                ArchivedIpv4Addr::check_bytes(
                    bytes.add(offset_of!(ArchivedIpAddrVariantV4, 1)).cast(),
                    context,
                )
                .map_err(ArchivedIpAddrError::InvalidIpv4Addr)?;
            }
            ArchivedIpAddrTag::TAG_V6 => {
                ArchivedIpv6Addr::check_bytes(
                    bytes.add(offset_of!(ArchivedIpAddrVariantV6, 1)).cast(),
                    context,
                )
                .map_err(ArchivedIpAddrError::InvalidIpv6Addr)?;
            }
            _ => return Err(ArchivedIpAddrError::InvalidTag(tag)),
        }
        Ok(&*value)
    }
}

/// Errors that can occur while checking an [`ArchivedSocketAddr`].
#[derive(Debug)]
pub enum ArchivedSocketAddrError {
    /// The option had an invalid tag
    InvalidTag(u8),
    InvalidSocketAddrV4(<ArchivedSocketAddrV4 as CheckBytes<()>>::Error),
    InvalidSocketAddrV6(<ArchivedSocketAddrV6 as CheckBytes<()>>::Error),
}

impl fmt::Display for ArchivedSocketAddrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchivedSocketAddrError::InvalidTag(tag) => {
                write!(f, "archived IP address had invalid tag: {}", tag)
            }
            ArchivedSocketAddrError::InvalidSocketAddrV4(e) => {
                write!(f, "archived IPv4 address check error: {}", e)
            }
            ArchivedSocketAddrError::InvalidSocketAddrV6(e) => {
                write!(f, "archived IPv6 address check error: {}", e)
            }
        }
    }
}

impl Error for ArchivedSocketAddrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ArchivedSocketAddrError::InvalidTag(_) => None,
            ArchivedSocketAddrError::InvalidSocketAddrV4(e) => Some(e as &dyn Error),
            ArchivedSocketAddrError::InvalidSocketAddrV6(e) => Some(e as &dyn Error),
        }
    }
}

impl From<Unreachable> for ArchivedSocketAddrError {
    fn from(_: Unreachable) -> Self {
        unsafe { core::hint::unreachable_unchecked() }
    }
}

impl ArchivedSocketAddrTag {
    const TAG_V4: u8 = ArchivedSocketAddrTag::V4 as u8;
    const TAG_V6: u8 = ArchivedSocketAddrTag::V6 as u8;
}

impl<C: ?Sized> CheckBytes<C> for ArchivedSocketAddr {
    type Error = ArchivedSocketAddrError;

    unsafe fn check_bytes<'a>(
        value: *const Self,
        context: &mut C,
    ) -> Result<&'a Self, Self::Error> {
        let bytes = value.cast::<u8>();
        let tag = *u8::check_bytes(bytes, context)?;
        match tag {
            ArchivedSocketAddrTag::TAG_V4 => {
                ArchivedSocketAddrV4::check_bytes(
                    bytes.add(offset_of!(ArchivedSocketAddrVariantV4, 1)).cast(),
                    context,
                )
                .map_err(ArchivedSocketAddrError::InvalidSocketAddrV4)?;
            }
            ArchivedSocketAddrTag::TAG_V6 => {
                ArchivedSocketAddrV6::check_bytes(
                    bytes.add(offset_of!(ArchivedSocketAddrVariantV6, 1)).cast(),
                    context,
                )
                .map_err(ArchivedSocketAddrError::InvalidSocketAddrV6)?;
            }
            _ => return Err(ArchivedSocketAddrError::InvalidTag(tag)),
        }
        Ok(&*value)
    }
}
