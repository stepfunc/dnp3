mod master;
mod outstation;

pub use master::*;
pub use outstation::*;

use tokio_rustls::rustls;

/// Determines how the certificate(s) presented by the peer are validated
///
/// This validation always occurs **after** the handshake signature has been
/// verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateMode {
    /// Validates the peer certificate against one or more configured trust anchors
    ///
    /// This mode uses the default certificate verifier in `rustls` to ensure that
    /// the chain of certificates presented by the peer is valid against one of
    /// the configured trust anchors.
    ///
    /// The name verification is relaxed to allow for certificates that do not contain
    /// the SAN extension. In these cases the name is verified using the Common Name instead.
    AuthorityBased,
    /// Validates that the peer presents a single certificate which is a byte-for-byte match
    /// against the configured peer certificate.
    ///
    /// The certificate is parsed only to ensure that the `NotBefore` and `NotAfter`
    /// are valid for the current system time.
    SelfSigned,
}

/// TLS-related errors
#[derive(Debug)]
pub enum TlsError {
    /// Invalid peer certificate
    InvalidPeerCertificate(std::io::Error),
    /// Invalid local certificate
    InvalidLocalCertificate(std::io::Error),
    /// Invalid private key
    InvalidPrivateKey(std::io::Error),
    /// DNS name is invalid
    InvalidDnsName,
    /// Other error
    Other(std::io::Error),
}

impl From<webpki::Error> for TlsError {
    fn from(value: webpki::Error) -> Self {
        TlsError::Other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            value.to_string(),
        ))
    }
}

impl From<std::io::Error> for TlsError {
    fn from(err: std::io::Error) -> Self {
        Self::Other(err)
    }
}

impl From<rustls::Error> for TlsError {
    fn from(value: rustls::Error) -> Self {
        Self::Other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            value.to_string(),
        ))
    }
}

impl From<sfio_rustls_util::Error> for TlsError {
    fn from(err: sfio_rustls_util::Error) -> Self {
        Self::Other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        ))
    }
}

impl From<rustls::client::InvalidDnsNameError> for TlsError {
    fn from(_: rustls::client::InvalidDnsNameError) -> Self {
        Self::InvalidDnsName
    }
}

impl From<pem_util::Error> for TlsError {
    fn from(value: pem_util::Error) -> Self {
        match value {
            pem_util::Error::InvalidPem(x) => Self::Other(x),
            pem_util::Error::DecryptionError(x) => Self::Other(x),
            pem_util::Error::NoPrivateKey(_) => Self::Other(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No private key",
            )),
            pem_util::Error::MoreThanOnePrivateKey(_) => Self::Other(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "More than one private key",
            )),
            pem_util::Error::NoCertificate => Self::Other(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "No certificate in PEM",
            )),
        }
    }
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPeerCertificate(err) => {
                write!(f, "invalid peer certificate file: {err}")
            }
            Self::InvalidLocalCertificate(err) => {
                write!(f, "invalid local certificate file: {err}")
            }
            Self::InvalidPrivateKey(err) => write!(f, "invalid private key file: {err}"),
            Self::InvalidDnsName => write!(f, "invalid DNS name"),
            Self::Other(err) => write!(f, "miscellaneous TLS error: {err}"),
        }
    }
}

impl std::error::Error for TlsError {}

/// Minimum TLS version to allow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinTlsVersion {
    /// TLS 1.2
    V12,
    /// TLS 1.3
    V13,
}

impl MinTlsVersion {
    fn to_rustls(self) -> &'static [&'static rustls::SupportedProtocolVersion] {
        static MIN_TLS12_VERSIONS: &[&rustls::SupportedProtocolVersion] =
            &[&rustls::version::TLS13, &rustls::version::TLS12];
        static MIN_TLS13_VERSIONS: &[&rustls::SupportedProtocolVersion] =
            &[&rustls::version::TLS13];

        match self {
            Self::V12 => MIN_TLS12_VERSIONS,
            Self::V13 => MIN_TLS13_VERSIONS,
        }
    }
}

pub(crate) fn expect_single_peer_cert(
    peer_certs: Vec<rustls::Certificate>,
) -> Result<rustls::Certificate, std::io::Error> {
    let mut iter = peer_certs.into_iter();
    let first = match iter.next() {
        None => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "no peer certificate",
            ))
        }
        Some(x) => x,
    };

    if iter.next().is_some() {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "more than one peer certificate in self-signed mode",
        ))
    } else {
        Ok(first)
    }
}

/// Configuration options related to dangerous (less-secure) modes that can only be used
/// after explicitly opting into them.
pub mod dangerous {

    use sfio_rustls_util::NameVerifier;
    use std::sync::atomic::Ordering;

    pub(crate) fn verifier(name: &str) -> NameVerifier {
        if name == "*" && ALLOW_WILDCARD_PEER_NAMES.load(Ordering::Relaxed) {
            NameVerifier::any()
        } else {
            NameVerifier::equal_to(name.to_string())
        }
    }

    static ALLOW_WILDCARD_PEER_NAMES: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// Setting this option will allow the user to specify an "*" for the peer's name
    /// when creating a [crate::tcp::tls::TlsServerConfig]. This means that the any certificate
    /// signed by the CA will be allowed
    pub fn enable_peer_name_wildcards() {
        ALLOW_WILDCARD_PEER_NAMES.store(true, Ordering::Relaxed);
    }
}
