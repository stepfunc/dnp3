mod master;
mod outstation;

pub use master::*;
pub use outstation::*;

use tokio_rustls::rustls::pki_types;

/// Determines how the certificate(s) presented by the peer are validated
///
/// This validation always occurs **after** the handshake signature has been
/// verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
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

impl From<sfio_rustls_config::Error> for TlsError {
    fn from(err: sfio_rustls_config::Error) -> Self {
        Self::Other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            err.to_string(),
        ))
    }
}

impl From<pki_types::InvalidDnsNameError> for TlsError {
    fn from(_: pki_types::InvalidDnsNameError) -> Self {
        Self::InvalidDnsName
    }
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
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
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub enum MinTlsVersion {
    /// Allow TLS 1.2 and TLS 1.3
    V12,
    /// Allow only TLS 1.3
    ///
    /// A peer that does not support TLS 1.3 will fail to complete the handshake.
    V13,
}

impl From<MinTlsVersion> for sfio_rustls_config::ProtocolVersions {
    fn from(value: MinTlsVersion) -> Self {
        match value {
            MinTlsVersion::V12 => sfio_rustls_config::ProtocolVersions::new()
                .enable_v12()
                .enable_v13(),
            MinTlsVersion::V13 => sfio_rustls_config::ProtocolVersions::v13_only(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sfio_rustls_config::ProtocolVersions;

    /// `MinTlsVersion` is a *minimum*, so v1.2 must also allow v1.3 to be negotiated.
    #[test]
    fn v12_allows_both_versions() {
        assert_eq!(
            ProtocolVersions::from(MinTlsVersion::V12),
            ProtocolVersions::new().enable_v12().enable_v13()
        );
    }

    /// Requesting v1.3 must *exclude* v1.2 so that the handshake fails closed against
    /// a peer that only supports v1.2.
    #[test]
    fn v13_excludes_v12() {
        assert_eq!(
            ProtocolVersions::from(MinTlsVersion::V13),
            ProtocolVersions::v13_only()
        );
        assert_ne!(
            ProtocolVersions::from(MinTlsVersion::V13),
            ProtocolVersions::from(MinTlsVersion::V12)
        );
    }
}
