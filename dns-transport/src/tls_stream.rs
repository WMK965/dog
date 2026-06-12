use std::convert::TryFrom;
use std::net::TcpStream;
use std::sync::Arc;

use super::Error;
use super::HttpsTransport;
use super::TlsTransport;

#[cfg(feature = "with_rustls")]
fn stream_rustls(domain: &str, port: u16) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, Error> {
    use rustls::pki_types::ServerName;

    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let server_name: ServerName<'_> = ServerName::try_from(domain)?.to_owned();

    let conn = rustls::ClientConnection::new(Arc::new(config), server_name)?;

    let sock = TcpStream::connect((domain, port))?;
    let tls = rustls::StreamOwned::new(conn, sock);

    Ok(tls)
}

pub trait TlsStream<S: std::io::Read + std::io::Write> {
    fn stream(domain: &str, port: u16) -> Result<S, Error>;
}

#[cfg(any(feature = "with_tls", feature = "with_https"))]
cfg_if::cfg_if! {
    if #[cfg(feature = "with_rustls")] {

        impl TlsStream<rustls::StreamOwned<rustls::ClientConnection, TcpStream>> for HttpsTransport {
            fn stream(domain: &str, port: u16) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, Error> {
                stream_rustls(domain, port)
            }
        }

        impl TlsStream<rustls::StreamOwned<rustls::ClientConnection, TcpStream>> for TlsTransport {
            fn stream(domain: &str, port: u16) -> Result<rustls::StreamOwned<rustls::ClientConnection, TcpStream>, Error> {
                stream_rustls(domain, port)
            }
        }

    } else {
        unreachable!("tls/https enabled but no tls implementation provided")
    }
}
