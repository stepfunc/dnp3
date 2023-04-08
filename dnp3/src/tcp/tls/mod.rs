mod master;
mod name;
mod outstation;

pub use master::*;
pub use name::*;
pub use outstation::*;

use tokio_rustls::{rustls, webpki};

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

struct SelfSignedVerifier {
    // expected certificate
    expected_peer_cert: rustls::Certificate,
}

impl SelfSignedVerifier {
    pub(crate) fn new(expected: rustls::Certificate) -> Self {
        Self {
            expected_peer_cert: expected,
        }
    }

    pub(crate) fn verify(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        now: std::time::SystemTime,
    ) -> Result<(), rustls::Error> {
        // Check that no intermediate certificates are present
        if !intermediates.is_empty() {
            return Err(rustls::Error::General(format!(
                "client sent {} intermediate certificates, expected none",
                intermediates.len()
            )));
        }

        // Check that presented certificate matches byte-for-byte the expected certificate
        if end_entity != &self.expected_peer_cert {
            return Err(rustls::Error::InvalidCertificateData(
                "client certificate doesn't match the expected self-signed certificate".to_string(),
            ));
        }

        // Check that the certificate is still valid
        let parsed_cert = rx509::x509::Certificate::parse(&end_entity.0).map_err(|err| {
            rustls::Error::InvalidCertificateData(format!(
                "unable to parse cert with rasn: {err:?}"
            ))
        })?;

        let now = now
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| rustls::Error::FailedToGetCurrentTime)?;
        let now = rx509::der::UtcTime::from_seconds_since_epoch(now.as_secs());

        if !parsed_cert.tbs_certificate.value.validity.is_valid(now) {
            return Err(rustls::Error::InvalidCertificateData(
                "self-signed certificate is currently not valid".to_string(),
            ));
        }

        // We do not validate DNS name. Providing the exact same certificate is sufficient.

        Ok(())
    }
}

pub(crate) fn pki_error(error: webpki::Error) -> rustls::Error {
    use webpki::Error::*;
    match error {
        BadDer | BadDerTime => rustls::Error::InvalidCertificateEncoding,
        InvalidSignatureForPublicKey => rustls::Error::InvalidCertificateSignature,
        UnsupportedSignatureAlgorithm | UnsupportedSignatureAlgorithmForPublicKey => {
            rustls::Error::InvalidCertificateSignatureType
        }
        e => rustls::Error::InvalidCertificateData(format!("invalid peer certificate: {e}")),
    }
}
