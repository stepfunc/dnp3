mod master;
mod outstation;

use std::io::{self, ErrorKind};
use std::path::Path;

pub use master::*;
pub use outstation::*;
use tokio_rustls::rustls;

/// TLS-related errors
#[derive(Debug)]
pub enum TlsError {
    /// Invalid peer certificate
    InvalidPeerCertificate(io::Error),
    /// Invalid local certificate
    InvalidLocalCertificate(io::Error),
    /// Invalid private key
    InvalidPrivateKey(io::Error),
    /// DNS name is invalid
    InvalidDnsName,
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPeerCertificate(err) => {
                write!(f, "invalid peer certificate file: {}", err)
            }
            Self::InvalidLocalCertificate(err) => {
                write!(f, "invalid local certificate file: {}", err)
            }
            Self::InvalidPrivateKey(err) => write!(f, "invalid private key file: {}", err),
            Self::InvalidDnsName => write!(f, "invalid DNS name"),
        }
    }
}

impl std::error::Error for TlsError {}

/// Minimum TLS version to allow
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MinTlsVersion {
    /// TLS 1.2
    Tls1_2,
    /// TLS 1.3
    Tls1_3,
}

impl MinTlsVersion {
    fn to_vec(self) -> Vec<rustls::ProtocolVersion> {
        match self {
            Self::Tls1_2 => vec![
                rustls::ProtocolVersion::TLSv1_3,
                rustls::ProtocolVersion::TLSv1_2,
            ],
            Self::Tls1_3 => vec![rustls::ProtocolVersion::TLSv1_3],
        }
    }
}

fn load_certs(path: &Path, is_local: bool) -> Result<Vec<rustls::Certificate>, TlsError> {
    let map_error_fn = match is_local {
        false => |err| TlsError::InvalidPeerCertificate(err),
        true => |err| TlsError::InvalidLocalCertificate(err),
    };

    let f = std::fs::File::open(path).map_err(map_error_fn)?;
    let mut f = io::BufReader::new(f);

    let certs = rustls_pemfile::certs(&mut f)
        .map_err(map_error_fn)?
        .iter()
        .map(|data| rustls::Certificate(data.clone()))
        .collect::<Vec<_>>();

    match certs.len() {
        0 => Err(map_error_fn(io::Error::new(
            ErrorKind::InvalidData,
            "no certificate in pem file",
        ))),
        _ => Ok(certs),
    }
}

fn load_private_key(path: &Path) -> Result<rustls::PrivateKey, TlsError> {
    let f = std::fs::File::open(path).map_err(TlsError::InvalidPrivateKey)?;
    let mut f = io::BufReader::new(f);

    match rustls_pemfile::read_one(&mut f).map_err(TlsError::InvalidPrivateKey)? {
        Some(rustls_pemfile::Item::RSAKey(key)) => Ok(rustls::PrivateKey(key)),
        Some(rustls_pemfile::Item::PKCS8Key(key)) => Ok(rustls::PrivateKey(key)),
        Some(rustls_pemfile::Item::X509Certificate(_)) => {
            Err(TlsError::InvalidPrivateKey(io::Error::new(
                ErrorKind::InvalidData,
                "file contains cert, not private key",
            )))
        }
        None => Err(TlsError::InvalidPrivateKey(io::Error::new(
            ErrorKind::InvalidData,
            "file does not contain private key",
        ))),
    }
}
