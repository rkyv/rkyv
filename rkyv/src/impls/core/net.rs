use core::{
    cmp,
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use munge::munge;
use rancor::Fallible;

use crate::{
    net::{
        ArchivedIpAddr, ArchivedIpv4Addr, ArchivedIpv6Addr, ArchivedSocketAddr,
        ArchivedSocketAddrV4, ArchivedSocketAddrV6,
    },
    traits::NoUndef,
    Archive, Deserialize, Place, Serialize,
};

// Ipv4Addr

impl Archive for Ipv4Addr {
    type Archived = ArchivedIpv4Addr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedIpv4Addr::emplace(self.octets(), out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Ipv4Addr {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Ipv4Addr, D> for ArchivedIpv4Addr {
    fn deserialize(&self, _: &mut D) -> Result<Ipv4Addr, D::Error> {
        Ok(self.as_ipv4())
    }
}

impl PartialEq<Ipv4Addr> for ArchivedIpv4Addr {
    #[inline]
    fn eq(&self, other: &Ipv4Addr) -> bool {
        self.as_ipv4().eq(other)
    }
}

impl PartialEq<ArchivedIpv4Addr> for Ipv4Addr {
    #[inline]
    fn eq(&self, other: &ArchivedIpv4Addr) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<Ipv4Addr> for ArchivedIpv4Addr {
    #[inline]
    fn partial_cmp(&self, other: &Ipv4Addr) -> Option<cmp::Ordering> {
        self.as_ipv4().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedIpv4Addr> for Ipv4Addr {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedIpv4Addr) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

// Ipv6Addr

impl Archive for Ipv6Addr {
    type Archived = ArchivedIpv6Addr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedIpv6Addr::emplace(self.octets(), out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for Ipv6Addr {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<Ipv6Addr, D> for ArchivedIpv6Addr {
    fn deserialize(&self, _: &mut D) -> Result<Ipv6Addr, D::Error> {
        Ok(self.as_ipv6())
    }
}

impl PartialEq<Ipv6Addr> for ArchivedIpv6Addr {
    #[inline]
    fn eq(&self, other: &Ipv6Addr) -> bool {
        self.as_ipv6().eq(other)
    }
}

impl PartialEq<ArchivedIpv6Addr> for Ipv6Addr {
    #[inline]
    fn eq(&self, other: &ArchivedIpv6Addr) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<Ipv6Addr> for ArchivedIpv6Addr {
    #[inline]
    fn partial_cmp(&self, other: &Ipv6Addr) -> Option<cmp::Ordering> {
        self.as_ipv6().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedIpv6Addr> for Ipv6Addr {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedIpv6Addr) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

// IpAddr

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedIpAddrTag {
    V4,
    V6,
}

// SAFETY: `ArchivedIpArrdTag` is `repr(u8)` and so always consists of a single
// well-defined byte.
unsafe impl NoUndef for ArchivedIpAddrTag {}

#[repr(C)]
struct ArchivedIpAddrVariantV4(ArchivedIpAddrTag, ArchivedIpv4Addr);

#[repr(C)]
struct ArchivedIpAddrVariantV6(ArchivedIpAddrTag, ArchivedIpv6Addr);

impl Archive for IpAddr {
    type Archived = ArchivedIpAddr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        match self {
            IpAddr::V4(ipv4_addr) => {
                let out =
                    unsafe { out.cast_unchecked::<ArchivedIpAddrVariantV4>() };
                munge!(let ArchivedIpAddrVariantV4(tag, out_ipv4_addr) = out);
                tag.write(ArchivedIpAddrTag::V4);
                ArchivedIpv4Addr::emplace(ipv4_addr.octets(), out_ipv4_addr);
            }
            IpAddr::V6(ipv6_addr) => {
                let out =
                    unsafe { out.cast_unchecked::<ArchivedIpAddrVariantV6>() };
                munge!(let ArchivedIpAddrVariantV6(tag, out_ipv6_addr) = out);
                tag.write(ArchivedIpAddrTag::V6);
                ArchivedIpv6Addr::emplace(ipv6_addr.octets(), out_ipv6_addr);
            }
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for IpAddr {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        match self {
            IpAddr::V4(ipv4_addr) => ipv4_addr.serialize(serializer),
            IpAddr::V6(ipv6_addr) => ipv6_addr.serialize(serializer),
        }
    }
}

impl<D: Fallible + ?Sized> Deserialize<IpAddr, D> for ArchivedIpAddr {
    fn deserialize(&self, deserializer: &mut D) -> Result<IpAddr, D::Error> {
        match self {
            ArchivedIpAddr::V4(ipv4_addr) => {
                Ok(IpAddr::V4(ipv4_addr.deserialize(deserializer)?))
            }
            ArchivedIpAddr::V6(ipv6_addr) => {
                Ok(IpAddr::V6(ipv6_addr.deserialize(deserializer)?))
            }
        }
    }
}

impl PartialEq<IpAddr> for ArchivedIpAddr {
    #[inline]
    fn eq(&self, other: &IpAddr) -> bool {
        match self {
            ArchivedIpAddr::V4(self_ip) => {
                if let IpAddr::V4(other_ip) = other {
                    self_ip.eq(other_ip)
                } else {
                    false
                }
            }
            ArchivedIpAddr::V6(self_ip) => {
                if let IpAddr::V6(other_ip) = other {
                    self_ip.eq(other_ip)
                } else {
                    false
                }
            }
        }
    }
}

impl PartialEq<ArchivedIpAddr> for IpAddr {
    #[inline]
    fn eq(&self, other: &ArchivedIpAddr) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<IpAddr> for ArchivedIpAddr {
    #[inline]
    fn partial_cmp(&self, other: &IpAddr) -> Option<cmp::Ordering> {
        self.as_ipaddr().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedIpAddr> for IpAddr {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedIpAddr) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

// SocketAddrV4

impl Archive for SocketAddrV4 {
    type Archived = ArchivedSocketAddrV4;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedSocketAddrV4::emplace(self, out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for SocketAddrV4 {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D> Deserialize<SocketAddrV4, D> for ArchivedSocketAddrV4
where
    D: Fallible + ?Sized,
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<SocketAddrV4, D::Error> {
        let ip = self.ip().deserialize(deserializer)?;
        Ok(SocketAddrV4::new(ip, self.port()))
    }
}

impl PartialEq<SocketAddrV4> for ArchivedSocketAddrV4 {
    #[inline]
    fn eq(&self, other: &SocketAddrV4) -> bool {
        self.as_socket_addr_v4().eq(other)
    }
}

impl PartialEq<ArchivedSocketAddrV4> for SocketAddrV4 {
    #[inline]
    fn eq(&self, other: &ArchivedSocketAddrV4) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<SocketAddrV4> for ArchivedSocketAddrV4 {
    #[inline]
    fn partial_cmp(&self, other: &SocketAddrV4) -> Option<cmp::Ordering> {
        self.as_socket_addr_v4().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedSocketAddrV4> for SocketAddrV4 {
    #[inline]
    fn partial_cmp(
        &self,
        other: &ArchivedSocketAddrV4,
    ) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

// SocketAddrV6

impl Archive for SocketAddrV6 {
    type Archived = ArchivedSocketAddrV6;
    type Resolver = ();

    #[inline]
    fn resolve(&self, _: Self::Resolver, out: Place<Self::Archived>) {
        ArchivedSocketAddrV6::emplace(self, out);
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for SocketAddrV6 {
    fn serialize(&self, _: &mut S) -> Result<Self::Resolver, S::Error> {
        Ok(())
    }
}

impl<D: Fallible + ?Sized> Deserialize<SocketAddrV6, D>
    for ArchivedSocketAddrV6
{
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<SocketAddrV6, D::Error> {
        let ip = self.ip().deserialize(deserializer)?;
        Ok(SocketAddrV6::new(
            ip,
            self.port(),
            self.flowinfo(),
            self.scope_id(),
        ))
    }
}

impl PartialEq<SocketAddrV6> for ArchivedSocketAddrV6 {
    #[inline]
    fn eq(&self, other: &SocketAddrV6) -> bool {
        self.as_socket_addr_v6().eq(other)
    }
}

impl PartialEq<ArchivedSocketAddrV6> for SocketAddrV6 {
    #[inline]
    fn eq(&self, other: &ArchivedSocketAddrV6) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<SocketAddrV6> for ArchivedSocketAddrV6 {
    #[inline]
    fn partial_cmp(&self, other: &SocketAddrV6) -> Option<cmp::Ordering> {
        self.as_socket_addr_v6().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedSocketAddrV6> for SocketAddrV6 {
    #[inline]
    fn partial_cmp(
        &self,
        other: &ArchivedSocketAddrV6,
    ) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

// SocketAddr

#[allow(dead_code)]
#[repr(u8)]
enum ArchivedSocketAddrTag {
    V4,
    V6,
}

// SAFETY: `ArchivedSocketAddrTag` is `repr(u8)` and so always consists of a
// single well-defined byte.
unsafe impl NoUndef for ArchivedSocketAddrTag {}

#[repr(C)]
struct ArchivedSocketAddrVariantV4(ArchivedSocketAddrTag, ArchivedSocketAddrV4);

#[repr(C)]
struct ArchivedSocketAddrVariantV6(ArchivedSocketAddrTag, ArchivedSocketAddrV6);

impl Archive for SocketAddr {
    type Archived = ArchivedSocketAddr;
    type Resolver = ();

    #[inline]
    fn resolve(&self, resolver: Self::Resolver, out: Place<Self::Archived>) {
        match self {
            SocketAddr::V4(socket_addr) => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedSocketAddrVariantV4>()
                };
                munge! {
                    let ArchivedSocketAddrVariantV4(tag, out_socket_addr) = out;
                }
                tag.write(ArchivedSocketAddrTag::V4);
                socket_addr.resolve(resolver, out_socket_addr);
            }
            SocketAddr::V6(socket_addr) => {
                let out = unsafe {
                    out.cast_unchecked::<ArchivedSocketAddrVariantV6>()
                };
                munge! {
                    let ArchivedSocketAddrVariantV6(tag, out_socket_addr) = out;
                }
                tag.write(ArchivedSocketAddrTag::V6);
                socket_addr.resolve(resolver, out_socket_addr);
            }
        }
    }
}

impl<S: Fallible + ?Sized> Serialize<S> for SocketAddr {
    fn serialize(
        &self,
        serializer: &mut S,
    ) -> Result<Self::Resolver, S::Error> {
        match self {
            SocketAddr::V4(socket_addr) => socket_addr.serialize(serializer),
            SocketAddr::V6(socket_addr) => socket_addr.serialize(serializer),
        }
    }
}

impl<D: Fallible + ?Sized> Deserialize<SocketAddr, D> for ArchivedSocketAddr {
    fn deserialize(
        &self,
        deserializer: &mut D,
    ) -> Result<SocketAddr, D::Error> {
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

impl PartialEq<SocketAddr> for ArchivedSocketAddr {
    #[inline]
    fn eq(&self, other: &SocketAddr) -> bool {
        self.as_socket_addr().eq(other)
    }
}

impl PartialEq<ArchivedSocketAddr> for SocketAddr {
    #[inline]
    fn eq(&self, other: &ArchivedSocketAddr) -> bool {
        other.eq(self)
    }
}

impl PartialOrd<SocketAddr> for ArchivedSocketAddr {
    #[inline]
    fn partial_cmp(&self, other: &SocketAddr) -> Option<cmp::Ordering> {
        self.as_socket_addr().partial_cmp(other)
    }
}

impl PartialOrd<ArchivedSocketAddr> for SocketAddr {
    #[inline]
    fn partial_cmp(&self, other: &ArchivedSocketAddr) -> Option<cmp::Ordering> {
        other.partial_cmp(self)
    }
}

#[cfg(test)]
mod tests {
    use core::net::{
        IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
    };

    use crate::api::test::roundtrip;

    #[test]
    fn roundtrip_ipv4_addr() {
        roundtrip(&Ipv4Addr::new(31, 41, 59, 26));
    }

    #[test]
    fn roundtrip_ipv6_addr() {
        roundtrip(&Ipv6Addr::new(31, 41, 59, 26, 53, 58, 97, 93));
    }

    #[test]
    fn roundtrip_ip_addr() {
        roundtrip(&IpAddr::V4(Ipv4Addr::new(31, 41, 59, 26)));
        roundtrip(&IpAddr::V6(Ipv6Addr::new(31, 41, 59, 26, 53, 58, 97, 93)));
    }

    #[test]
    fn roundtrip_socket_addr_v4() {
        roundtrip(&SocketAddrV4::new(Ipv4Addr::new(31, 41, 59, 26), 5358));
    }

    #[test]
    fn roundtrip_socket_addr_v6() {
        roundtrip(&SocketAddrV6::new(
            Ipv6Addr::new(31, 31, 59, 26, 53, 58, 97, 93),
            2384,
            0,
            0,
        ));
    }

    #[test]
    fn roundtrip_socket_addr() {
        roundtrip(&SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(31, 41, 59, 26),
            5358,
        )));
        roundtrip(&SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(31, 31, 59, 26, 53, 58, 97, 93),
            2384,
            0,
            0,
        )));
    }
}
