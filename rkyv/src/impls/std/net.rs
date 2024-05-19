use core::net::{SocketAddr, SocketAddrV4, SocketAddrV6};
use std::{io, net::ToSocketAddrs};

use crate::net::{
    ArchivedSocketAddr, ArchivedSocketAddrV4, ArchivedSocketAddrV6,
};

impl ToSocketAddrs for ArchivedSocketAddrV4 {
    type Iter = <SocketAddrV4 as ToSocketAddrs>::Iter;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        self.as_socket_addr_v4().to_socket_addrs()
    }
}

impl ToSocketAddrs for ArchivedSocketAddrV6 {
    type Iter = <SocketAddrV6 as ToSocketAddrs>::Iter;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        self.as_socket_addr_v6().to_socket_addrs()
    }
}

impl ToSocketAddrs for ArchivedSocketAddr {
    type Iter = <SocketAddr as ToSocketAddrs>::Iter;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        self.as_socket_addr().to_socket_addrs()
    }
}
