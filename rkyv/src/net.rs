//! Archived versions of network types.

use core::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
};

use munge::munge;

use crate::{
    primitive::{ArchivedU16, ArchivedU32},
    Archive, Place, Portable,
};

/// An archived [`Ipv4Addr`].
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedIpv4Addr {
    octets: [u8; 4],
}

impl ArchivedIpv4Addr {
    /// Returns the four eight-bit integers that make up this address.
    #[inline]
    pub const fn octets(&self) -> [u8; 4] {
        self.octets
    }

    /// Returns an [`Ipv4Addr`] with the same value.
    #[inline]
    pub const fn as_ipv4(&self) -> Ipv4Addr {
        let octets = self.octets();
        Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3])
    }

    /// Returns `true` if this is a broadcast address (255.255.255.255).
    ///
    /// See [`Ipv4Addr::is_broadcast`] for more details.
    #[inline]
    pub const fn is_broadcast(&self) -> bool {
        self.as_ipv4().is_broadcast()
    }

    /// Returns `true` if this address is in a range designated for
    /// documentation.
    ///
    /// See [`Ipv4Addr::is_documentation`] for more details.
    #[inline]
    pub const fn is_documentation(&self) -> bool {
        self.as_ipv4().is_documentation()
    }

    /// Returns `true` if the address is link-local (169.254.0.0/16).
    ///
    /// See [`Ipv4Addr::is_link_local`] for more details.
    #[inline]
    pub const fn is_link_local(&self) -> bool {
        self.as_ipv4().is_link_local()
    }

    /// Returns `true` if this is a loopback address (127.0.0.0/8).
    ///
    /// See [`Ipv4Addr::is_loopback`] for more details.
    #[inline]
    pub const fn is_loopback(&self) -> bool {
        self.as_ipv4().is_loopback()
    }

    /// Returns `true` if this is a multicast address (224.0.0.0/4).
    ///
    /// See [`Ipv4Addr::is_multicast`] for more details.
    #[inline]
    pub const fn is_multicast(&self) -> bool {
        self.as_ipv4().is_multicast()
    }

    /// Returns `true` if this is a private address.
    ///
    /// See [`Ipv4Addr::is_private`] for more details.
    #[inline]
    pub const fn is_private(&self) -> bool {
        self.as_ipv4().is_private()
    }

    /// Returns `true` for the special 'unspecified' address (0.0.0.0).
    ///
    /// See [`Ipv4Addr::is_unspecified`] for more details.
    #[inline]
    pub const fn is_unspecified(&self) -> bool {
        self.as_ipv4().is_unspecified()
    }

    /// Converts this address to an IPv4-compatible
    /// [`IPv6` address](std::net::Ipv6Addr).
    ///
    /// See [`Ipv4Addr::to_ipv6_compatible`] for more
    /// details.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub const fn to_ipv6_compatible(&self) -> Ipv6Addr {
        self.as_ipv4().to_ipv6_compatible()
    }

    /// Converts this address to an IPv4-mapped
    /// [`IPv6` address](std::net::Ipv6Addr).
    ///
    /// See [`Ipv4Addr::to_ipv6_mapped`] for more details.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub const fn to_ipv6_mapped(&self) -> Ipv6Addr {
        self.as_ipv4().to_ipv6_mapped()
    }

    /// Emplaces an `ArchivedIpv4Addr` with the given octets into a place.
    #[inline]
    pub fn emplace(octets: [u8; 4], out: Place<Self>) {
        unsafe {
            out.cast_unchecked::<[u8; 4]>().write(octets);
        }
    }
}

/// An archived [`Ipv6Addr`].
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ArchivedIpv6Addr {
    octets: [u8; 16],
}

impl ArchivedIpv6Addr {
    /// Returns the eight 16-bit segments that make up this address.
    #[inline]
    pub const fn segments(&self) -> [u16; 8] {
        [
            u16::from_be_bytes([self.octets[0], self.octets[1]]),
            u16::from_be_bytes([self.octets[2], self.octets[3]]),
            u16::from_be_bytes([self.octets[4], self.octets[5]]),
            u16::from_be_bytes([self.octets[6], self.octets[7]]),
            u16::from_be_bytes([self.octets[8], self.octets[9]]),
            u16::from_be_bytes([self.octets[10], self.octets[11]]),
            u16::from_be_bytes([self.octets[12], self.octets[13]]),
            u16::from_be_bytes([self.octets[14], self.octets[15]]),
        ]
    }

    /// Returns an [`Ipv6Addr`] with the same value.
    #[inline]
    pub const fn as_ipv6(&self) -> Ipv6Addr {
        let segments = self.segments();
        Ipv6Addr::new(
            segments[0],
            segments[1],
            segments[2],
            segments[3],
            segments[4],
            segments[5],
            segments[6],
            segments[7],
        )
    }

    /// Returns `true` if this is a loopback address (::1).
    ///
    /// See [`Ipv6Addr::is_loopback()`](std::net::Ipv6Addr::is_loopback()) for
    /// more details.
    #[inline]
    pub const fn is_loopback(&self) -> bool {
        self.as_ipv6().is_loopback()
    }

    /// Returns `true` if this is a multicast address (ff00::/8).
    ///
    /// See [`Ipv6Addr::is_multicast()`](std::net::Ipv6Addr::is_multicast()) for
    /// more details.
    #[inline]
    pub const fn is_multicast(&self) -> bool {
        self.as_ipv6().is_multicast()
    }

    /// Returns `true` for the special 'unspecified' address (::).
    ///
    /// See [`Ipv6Addr::is_unspecified()`](std::net::Ipv6Addr::is_unspecified())
    /// for more details.
    #[inline]
    pub const fn is_unspecified(&self) -> bool {
        self.as_ipv6().is_unspecified()
    }

    /// Returns the sixteen eight-bit integers the IPv6 address consists of.
    #[inline]
    pub const fn octets(&self) -> [u8; 16] {
        self.as_ipv6().octets()
    }

    /// Converts this address to an [`IPv4` address](std::net::Ipv4Addr).
    /// Returns [`None`] if this address is neither IPv4-compatible or
    /// IPv4-mapped.
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub const fn to_ipv4(&self) -> Option<Ipv4Addr> {
        self.as_ipv6().to_ipv4()
    }

    /// Emplaces an `ArchivedIpv6Addr` with the given octets into a place.
    #[inline]
    pub fn emplace(octets: [u8; 16], out: Place<Self>) {
        unsafe {
            out.cast_unchecked::<[u8; 16]>().write(octets);
        }
    }
}

/// An archived [`IpAddr`].
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ArchivedIpAddr {
    /// An IPv4 address.
    V4(ArchivedIpv4Addr),
    /// An IPv6 address.
    V6(ArchivedIpv6Addr),
}

impl ArchivedIpAddr {
    /// Returns `true` if this address is an [`IPv4`
    /// address](std::net::IpAddr::V4), and `false` otherwise.
    #[inline]
    pub const fn is_ipv4(&self) -> bool {
        matches!(self, ArchivedIpAddr::V4(_))
    }

    /// Returns `true` if this address is an [`IPv6`
    /// address](std::net::IpAddr::V6), and `false` otherwise.
    #[inline]
    pub const fn is_ipv6(&self) -> bool {
        matches!(self, ArchivedIpAddr::V6(_))
    }

    /// Returns an [`IpAddr`] with the same value.
    #[inline]
    pub const fn as_ipaddr(&self) -> IpAddr {
        match self {
            ArchivedIpAddr::V4(ipv4) => IpAddr::V4(ipv4.as_ipv4()),
            ArchivedIpAddr::V6(ipv6) => IpAddr::V6(ipv6.as_ipv6()),
        }
    }

    /// Returns `true` if this is a loopback address.
    ///
    /// See [`IpAddr::is_loopback()`](std::net::IpAddr::is_loopback()) for more
    /// details.
    #[inline]
    pub const fn is_loopback(&self) -> bool {
        match self {
            ArchivedIpAddr::V4(ip) => ip.is_loopback(),
            ArchivedIpAddr::V6(ip) => ip.is_loopback(),
        }
    }

    /// Returns `true` if this is a multicast address.
    ///
    /// See [`IpAddr::is_multicast()`](std::net::IpAddr::is_multicast()) for
    /// more details.
    #[inline]
    pub const fn is_multicast(&self) -> bool {
        match self {
            ArchivedIpAddr::V4(ip) => ip.is_multicast(),
            ArchivedIpAddr::V6(ip) => ip.is_multicast(),
        }
    }

    /// Returns `true` for the special 'unspecified' address.
    ///
    /// See [`IpAddr::is_unspecified()`](std::net::IpAddr::is_unspecified()) for
    /// more details.
    #[inline]
    pub const fn is_unspecified(&self) -> bool {
        match self {
            ArchivedIpAddr::V4(ip) => ip.is_unspecified(),
            ArchivedIpAddr::V6(ip) => ip.is_unspecified(),
        }
    }
}

/// An archived [`SocketAddrV4`].
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, Portable, PartialOrd,
)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedSocketAddrV4 {
    ip: ArchivedIpv4Addr,
    port: ArchivedU16,
}

impl ArchivedSocketAddrV4 {
    /// Returns the IP address associated with this socket address.
    #[inline]
    pub const fn ip(&self) -> &ArchivedIpv4Addr {
        &self.ip
    }

    /// Returns the port number associated with this socket address.
    #[inline]
    pub const fn port(&self) -> u16 {
        self.port.to_native()
    }

    /// Returns a [`SocketAddrV4`] with the same value.
    #[inline]
    pub fn as_socket_addr_v4(&self) -> SocketAddrV4 {
        SocketAddrV4::new(self.ip().as_ipv4(), self.port())
    }

    /// Emplaces an `ArchivedSocketAddrV4` of the given `value` into a place.
    #[inline]
    pub fn emplace(value: &SocketAddrV4, out: Place<Self>) {
        munge!(let ArchivedSocketAddrV4 { ip, port } = out);
        value.ip().resolve((), ip);
        value.port().resolve((), port);
    }
}

/// An archived [`SocketAddrV6`].
#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, Portable, PartialOrd,
)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[rkyv(crate)]
#[repr(C)]
pub struct ArchivedSocketAddrV6 {
    ip: ArchivedIpv6Addr,
    port: ArchivedU16,
    flowinfo: ArchivedU32,
    scope_id: ArchivedU32,
}

impl ArchivedSocketAddrV6 {
    /// Returns the flow information associated with this address.
    ///
    /// See [`SocketAddrV6::flowinfo()`](std::net::SocketAddrV6::flowinfo()) for
    /// more details.
    #[inline]
    pub const fn flowinfo(&self) -> u32 {
        self.flowinfo.to_native()
    }

    /// Returns the IP address associated with this socket address.
    #[inline]
    pub const fn ip(&self) -> &ArchivedIpv6Addr {
        &self.ip
    }

    /// Returns the port number associated with this socket address.
    #[inline]
    pub const fn port(&self) -> u16 {
        self.port.to_native()
    }

    /// Returns the scope ID associated with this address.
    ///
    /// See [`SocketAddrV6::scope_id()`](std::net::SocketAddrV6::scope_id()) for
    /// more details.
    #[inline]
    pub const fn scope_id(&self) -> u32 {
        self.scope_id.to_native()
    }

    /// Returns a [`SocketAddrV6`] with the same value.
    #[inline]
    pub fn as_socket_addr_v6(&self) -> SocketAddrV6 {
        SocketAddrV6::new(
            self.ip().as_ipv6(),
            self.port(),
            self.flowinfo(),
            self.scope_id(),
        )
    }

    /// Emplaces an `ArchivedSocketAddrV6` of the given `value` into a place.
    #[inline]
    pub fn emplace(value: &SocketAddrV6, out: Place<Self>) {
        munge!(let ArchivedSocketAddrV6 { ip, port, flowinfo, scope_id } = out);
        value.ip().resolve((), ip);
        value.port().resolve((), port);
        value.flowinfo().resolve((), flowinfo);
        value.scope_id().resolve((), scope_id);
    }
}

/// An archived [`SocketAddr`].
#[derive(Portable)]
#[rkyv(crate)]
#[cfg_attr(feature = "bytecheck", derive(bytecheck::CheckBytes))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(u8)]
pub enum ArchivedSocketAddr {
    /// An IPv4 socket address.
    V4(ArchivedSocketAddrV4),
    /// An IPv6 socket address.
    V6(ArchivedSocketAddrV6),
}

impl ArchivedSocketAddr {
    /// Returns the port number associated with this socket address.
    #[inline]
    pub fn port(&self) -> u16 {
        match self {
            ArchivedSocketAddr::V4(addr) => addr.port(),
            ArchivedSocketAddr::V6(addr) => addr.port(),
        }
    }

    /// Returns `true` if the [IP address](std::net::IpAddr) in this
    /// `ArchivedSocketAddr` is an [`IPv4` address](std::net::IpAddr::V4),
    /// and `false` otherwise.
    #[inline]
    pub fn is_ipv4(&self) -> bool {
        matches!(self, ArchivedSocketAddr::V4(_))
    }

    /// Returns `true` if the [IP address](std::net::IpAddr) in this
    /// `ArchivedSocketAddr` is an [`IPv6` address](std::net::IpAddr::V6),
    /// and `false` otherwise.
    #[inline]
    pub fn is_ipv6(&self) -> bool {
        matches!(self, ArchivedSocketAddr::V6(_))
    }

    /// Returns a [`SocketAddr`] with the same value.
    #[inline]
    pub fn as_socket_addr(&self) -> SocketAddr {
        match self {
            ArchivedSocketAddr::V4(addr) => {
                SocketAddr::V4(addr.as_socket_addr_v4())
            }
            ArchivedSocketAddr::V6(addr) => {
                SocketAddr::V6(addr.as_socket_addr_v6())
            }
        }
    }

    /// Returns the IP address associated with this socket address.
    #[inline]
    pub fn ip(&self) -> IpAddr {
        match self {
            ArchivedSocketAddr::V4(addr) => IpAddr::V4(addr.ip().as_ipv4()),
            ArchivedSocketAddr::V6(addr) => IpAddr::V6(addr.ip().as_ipv6()),
        }
    }
}
