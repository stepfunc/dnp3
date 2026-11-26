// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#[cfg(test)]
mod handshake_tests;
mod master;
mod outstation;

pub use master::*;
pub use outstation::*;

/// How long a TLS handshake may take, on either side of the connection, so that a peer that stalls
/// its handshake can't hold up whatever is waiting for it
pub(crate) const TLS_HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Why a TLS handshake didn't produce a session
#[derive(Debug)]
pub(crate) enum HandshakeError {
    /// The peer didn't complete the handshake within [`TLS_HANDSHAKE_TIMEOUT`]
    Timeout,
    /// The handshake failed
    Failed(std::io::Error),
}

impl HandshakeError {
    /// Log why the handshake with `peer` didn't produce a session
    pub(crate) fn log(&self, peer: std::net::SocketAddr) {
        match self {
            Self::Timeout => tracing::warn!(
                "TLS handshake with {peer} didn't complete within {} seconds",
                TLS_HANDSHAKE_TIMEOUT.as_secs()
            ),
            Self::Failed(err) => {
                tracing::warn!("failed to establish TLS session with {peer}: {err}")
            }
        }
    }
}

/// Run a handshake, giving up if it doesn't complete within [`TLS_HANDSHAKE_TIMEOUT`]
async fn handshake<S>(
    handshake: impl std::future::Future<Output = std::io::Result<S>>,
) -> Result<S, HandshakeError> {
    match tokio::time::timeout(TLS_HANDSHAKE_TIMEOUT, handshake).await {
        Err(_) => Err(HandshakeError::Timeout),
        Ok(Err(err)) => Err(HandshakeError::Failed(err)),
        Ok(Ok(stream)) => Ok(stream),
    }
}

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
pub struct TlsError(pub(crate) TlsErrorKind);

#[derive(Debug)]
pub(crate) enum TlsErrorKind {
    InvalidDnsName,
    Config(sfio_rustls_config::Error),
}

impl TlsError {
    pub(crate) fn invalid_dns_name() -> Self {
        Self(TlsErrorKind::InvalidDnsName)
    }

    pub(crate) fn config(err: sfio_rustls_config::Error) -> Self {
        Self(TlsErrorKind::Config(err))
    }
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            TlsErrorKind::InvalidDnsName => f.write_str("invalid DNS name"),
            TlsErrorKind::Config(err) => write!(f, "TLS configuration error: {err}"),
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
