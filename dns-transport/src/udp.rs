use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

use log::*;

use dns::{Request, Response};
use super::{Transport, Error};


/// The **UDP transport**, which sends DNS wire data inside a UDP datagram.
///
/// # References
///
/// - [RFC 1035 §4.2.1](https://tools.ietf.org/html/rfc1035) — Domain Names,
///   Implementation and Specification (November 1987)
pub struct UdpTransport {
    addr: String,
}

impl UdpTransport {

    /// Creates a new UDP transport that connects to the given host.
    pub fn new(addr: String) -> Self {
        Self { addr }
    }
}


impl Transport for UdpTransport {
    fn send(&self, request: &Request) -> Result<Response, Error> {
        info!("UDP connecting to {}:53", self.addr);

        let target = resolve_socket_addr(&self.addr, 53)?;

        let socket = match target {
            SocketAddr::V4(_) => UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
                .or_else(|_| UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0)))?,
            SocketAddr::V6(_) => UdpSocket::bind((Ipv6Addr::UNSPECIFIED, 0))
                .or_else(|_| UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)))?,
        };

        socket.connect(target)?;
        debug!("UDP connected");
        Self::send_recv(socket, request)
    }
}

impl UdpTransport {
    fn send_recv(socket: UdpSocket, request: &Request) -> Result<Response, Error> {
        let bytes_to_send = request.to_bytes().expect("failed to serialise request");

        info!("Sending {} bytes of data over UDP", bytes_to_send.len());
        let written_len = socket.send(&bytes_to_send)?;
        debug!("Wrote {} bytes", written_len);

        info!("Waiting to receive...");
        let mut buf = vec![0; 4096];
        let received_len = socket.recv(&mut buf)?;

        info!("Received {} bytes of data", received_len);
        let response = Response::from_bytes(&buf[.. received_len])?;
        Ok(response)
    }
}

/// Resolves an address string (with optional explicit port) into a `SocketAddr`.
/// Handles IPv4, bracketed IPv6, and bare IPv6 addresses.
fn resolve_socket_addr(addr: &str, default_port: u16) -> Result<SocketAddr, Error> {
    // Already a full SocketAddr (e.g. "1.2.3.4:53" or "[::1]:53")
    if let Ok(sa) = addr.parse::<SocketAddr>() {
        return Ok(sa);
    }

    // IPv6 without brackets (e.g. "fe80::1" or "fd12::1") — wrap in []
    if addr.contains(':') {
        let bracketed = format!("[{}]:{}", addr, default_port);
        if let Ok(sa) = bracketed.parse::<SocketAddr>() {
            return Ok(sa);
        }
    }

    // IPv4 without port (e.g. "1.2.3.4") or hostname
    let with_port = format!("{}:{}", addr, default_port);
    if let Ok(sa) = with_port.parse::<SocketAddr>() {
        return Ok(sa);
    }

    // Last resort: ask the OS to resolve (hostname)
    use std::net::ToSocketAddrs;
    if let Ok(mut addrs) = (addr, default_port).to_socket_addrs() {
        if let Some(sa) = addrs.next() {
            return Ok(sa);
        }
    }

    Err(std::io::Error::new(std::io::ErrorKind::AddrNotAvailable, "Cannot resolve address").into())
}
