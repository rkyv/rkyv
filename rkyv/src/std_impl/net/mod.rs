//! [`Archive`] implementations for network types.

#[cfg(feature = "validation")]
pub mod validation;

use crate::{offset_of, project_struct, Archive, Archived, Deserialize, Fallible, Serialize};
use core::mem::MaybeUninit;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

/// An archived [`Ipv4Addr`](std::net::Ipv4Addr).
#[cfg_attr(feature = "validation", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedIpv4Addr {
    octets: [u8; 4],
}

impl Archive for Ipv4Addr {
    type Archived = ArchivedIpv4Addr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            out.as_mut_ptr().cast::<[u8; 4]>().write(self.octets());
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Ipv4Addr {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Ipv4Addr, D> for ArchivedIpv4Addr {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<Ipv4Addr, D::Error> {
        Ok(Ipv4Addr::from(self.octets))
    }
}

/// An archived [`Ipv6Addr`](std::net::Ipv6Addr).
#[cfg_attr(feature = "validation", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedIpv6Addr {
    octets: [u8; 16],
}

impl Archive for Ipv6Addr {
    type Archived = ArchivedIpv6Addr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            out.as_mut_ptr().cast::<[u8; 16]>().write(self.octets());
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Ipv6Addr {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Ipv6Addr, D> for ArchivedIpv6Addr {
    #[inline]
    fn deserialize(&self, _: &mut D) -> Result<Ipv6Addr, D::Error> {
        Ok(Ipv6Addr::from(self.octets))
    }
}

/// An archived [`IpAddr`](std::net::IpAddr).
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ArchivedIpAddr {
    V4(ArchivedIpv4Addr),
    V6(ArchivedIpv6Addr),
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedIpAddrTag {
    V4,
    V6,
}

#[repr(C)]
struct ArchivedIpAddrVariantV4(ArchivedIpAddrTag, ArchivedIpv4Addr);

#[repr(C)]
struct ArchivedIpAddrVariantV6(ArchivedIpAddrTag, ArchivedIpv6Addr);

impl Archive for IpAddr {
    type Archived = ArchivedIpAddr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        match self {
            IpAddr::V4(ipv4_addr) => unsafe {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedIpAddrVariantV4>>();
                project_struct!(out: ArchivedIpAddrVariantV4 => 0: ArchivedIpAddrTag)
                    .as_mut_ptr()
                    .write(ArchivedIpAddrTag::V4);
                #[allow(clippy::unit_arg)]
                ipv4_addr.resolve(
                    pos + offset_of!(ArchivedIpAddrVariantV4, 1),
                    resolver,
                    project_struct!(out: ArchivedIpAddrVariantV4 => 1),
                );
            },
            IpAddr::V6(ipv6_addr) => unsafe {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedIpAddrVariantV6>>();
                project_struct!(out: ArchivedIpAddrVariantV6 => 0: ArchivedIpAddrTag)
                    .as_mut_ptr()
                    .write(ArchivedIpAddrTag::V6);
                #[allow(clippy::unit_arg)]
                ipv6_addr.resolve(
                    pos + offset_of!(ArchivedIpAddrVariantV6, 1),
                    resolver,
                    project_struct!(out: ArchivedIpAddrVariantV6 => 1),
                );
            },
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for IpAddr {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        match self {
            IpAddr::V4(ipv4_addr) => ipv4_addr.serialize(serializer),
            IpAddr::V6(ipv6_addr) => ipv6_addr.serialize(serializer),
        }
    }
}

impl<D: Fallible + ?Sized> Deserialize<IpAddr, D> for Archived<IpAddr> {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<IpAddr, D::Error> {
        match self {
            ArchivedIpAddr::V4(ipv4_addr) => Ok(IpAddr::V4(ipv4_addr.deserialize(deserializer)?)),
            ArchivedIpAddr::V6(ipv6_addr) => Ok(IpAddr::V6(ipv6_addr.deserialize(deserializer)?)),
        }
    }
}

/// An archived [`SocketAddrV4`](std::net::SocketAddrV4).
#[cfg_attr(feature = "validation", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "strict", repr(C))]
pub struct ArchivedSocketAddrV4 {
    ip: ArchivedIpv4Addr,
    port: u16,
}

impl Archive for SocketAddrV4 {
    type Archived = ArchivedSocketAddrV4;
    type Resolver = ();

    #[inline]
    fn resolve(&self, pos: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            self.ip().resolve(
                pos + offset_of!(ArchivedSocketAddrV4, ip),
                (),
                project_struct!(out: Self::Archived => ip),
            );
            self.port().resolve(
                pos + offset_of!(ArchivedSocketAddrV4, port),
                (),
                project_struct!(out: Self::Archived => port),
            )
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for SocketAddrV4 {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<SocketAddrV4, D> for ArchivedSocketAddrV4 {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<SocketAddrV4, D::Error> {
        let ip = self.ip.deserialize(deserializer)?;
        Ok(SocketAddrV4::new(ip, self.port))
    }
}

/// An archived [`SocketAddrV6`](std::net::SocketAddrV6).
#[cfg_attr(feature = "validation", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "strict", repr(C))]
pub struct ArchivedSocketAddrV6 {
    ip: ArchivedIpv6Addr,
    port: u16,
    flowinfo: u32,
    scope_id: u32,
}

impl Archive for SocketAddrV6 {
    type Archived = ArchivedSocketAddrV6;
    type Resolver = ();

    #[inline]
    fn resolve(&self, pos: usize, _: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        unsafe {
            self.ip().resolve(
                pos + offset_of!(ArchivedSocketAddrV6, ip),
                (),
                project_struct!(out: Self::Archived => ip),
            );
            self.port().resolve(
                pos + offset_of!(ArchivedSocketAddrV6, port),
                (),
                project_struct!(out: Self::Archived => port),
            );
            self.flowinfo().resolve(
                pos + offset_of!(ArchivedSocketAddrV6, flowinfo),
                (),
                project_struct!(out: Self::Archived => flowinfo),
            );
            self.scope_id().resolve(
                pos + offset_of!(ArchivedSocketAddrV6, scope_id),
                (),
                project_struct!(out: Self::Archived => scope_id),
            )
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for SocketAddrV6 {
    #[inline]
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<SocketAddrV6, D> for ArchivedSocketAddrV6 {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<SocketAddrV6, D::Error> {
        let ip = self.ip.deserialize(deserializer)?;
        Ok(SocketAddrV6::new(
            ip,
            self.port,
            self.flowinfo,
            self.scope_id,
        ))
    }
}

/// An archived [`SocketAddr`](std::net::SocketAddr).
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ArchivedSocketAddr {
    V4(ArchivedSocketAddrV4),
    V6(ArchivedSocketAddrV6),
}

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedSocketAddrTag {
    V4,
    V6,
}

#[repr(C)]
struct ArchivedSocketAddrVariantV4(ArchivedSocketAddrTag, ArchivedSocketAddrV4);

#[repr(C)]
struct ArchivedSocketAddrVariantV6(ArchivedSocketAddrTag, ArchivedSocketAddrV6);

impl Archive for SocketAddr {
    type Archived = ArchivedSocketAddr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, pos: usize, resolver: Self::Resolver, out: &mut MaybeUninit<Self::Archived>) {
        match self {
            SocketAddr::V4(socket_addr) => unsafe {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedSocketAddrVariantV4>>();
                project_struct!(out: ArchivedSocketAddrVariantV4 => 0: ArchivedSocketAddrTag)
                    .as_mut_ptr()
                    .write(ArchivedSocketAddrTag::V4);
                #[allow(clippy::unit_arg)]
                socket_addr.resolve(
                    pos + offset_of!(ArchivedSocketAddrVariantV4, 1),
                    resolver,
                    project_struct!(out: ArchivedSocketAddrVariantV4 => 1),
                );
            },
            SocketAddr::V6(socket_addr) => unsafe {
                let out = &mut *out
                    .as_mut_ptr()
                    .cast::<MaybeUninit<ArchivedSocketAddrVariantV6>>();
                project_struct!(out: ArchivedSocketAddrVariantV6 => 0: ArchivedSocketAddrTag)
                    .as_mut_ptr()
                    .write(ArchivedSocketAddrTag::V6);
                #[allow(clippy::unit_arg)]
                socket_addr.resolve(
                    pos + offset_of!(ArchivedSocketAddrVariantV6, 1),
                    resolver,
                    project_struct!(out: ArchivedSocketAddrVariantV6 => 1),
                );
            },
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for SocketAddr {
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        match self {
            SocketAddr::V4(socket_addr) => socket_addr.serialize(serializer),
            SocketAddr::V6(socket_addr) => socket_addr.serialize(serializer),
        }
    }
}

impl<D: Fallible + ?Sized> Deserialize<SocketAddr, D> for Archived<SocketAddr> {
    #[inline]
    fn deserialize(&self, deserializer: &mut D) -> Result<SocketAddr, D::Error> {
        match self {
            ArchivedSocketAddr::V4(socket_addr) => {
                Ok(SocketAddr::V4(socket_addr.deserialize(deserializer)?))
            }
            ArchivedSocketAddr::V6(socket_addr) => {
                Ok(SocketAddr::V6(socket_addr.deserialize(deserializer)?))
            }
        }
    }
}
