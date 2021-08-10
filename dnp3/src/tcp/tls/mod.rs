use std::io;
mod master;

pub use master::*;

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
