use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::Arc;

use tokio_rustls::rustls::server::AllowAnyAuthenticatedClient;
use tokio_rustls::{rustls, webpki};

use crate::tcp::tls::{load_certs, load_private_key, CertificateMode, MinTlsVersion, TlsError};
use crate::tokio::net::TcpStream;
use crate::util::phys::PhysLayer;
use rasn;

/// TLS configuration
pub struct TlsServerConfig {
    config: Arc<rustls::ServerConfig>,
}

impl TlsServerConfig {
    /// Create a TLS server config
    pub fn new(
        name: &str,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        min_tls_version: MinTlsVersion,
        certificate_mode: CertificateMode,
    ) -> Result<Self, TlsError> {
        let mut peer_certs = load_certs(peer_cert_path, false)?;
        let local_certs = load_certs(local_cert_path, true)?;
        let private_key = load_private_key(private_key_path)?;

        let verifier = match certificate_mode {
            CertificateMode::TrustChain => {
                // Build root certificate store
                let mut roots = rustls::RootCertStore::empty();
                for cert in &peer_certs {
                    roots.add(cert).map_err(|err| {
                        TlsError::InvalidPeerCertificate(io::Error::new(
                            ErrorKind::InvalidData,
                            err.to_string(),
                        ))
                    })?;
                }

                let dns_name = webpki::DnsNameRef::try_from_ascii_str(name)
                    .map_err(|_| TlsError::InvalidDnsName)?
                    .to_owned();

                CaChainClientCertVerifier::new(roots, dns_name)
            }
            CertificateMode::SelfSignedCertificate => {
                if let Some(peer_cert) = peer_certs.pop() {
                    if !peer_certs.is_empty() {
                        return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                            ErrorKind::InvalidData,
                            "more than one peer certificate in self-signed mode",
                        )));
                    }

                    SelfSignedCertificateClientCertVerifier::new(peer_cert)
                } else {
                    return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                        ErrorKind::InvalidData,
                        "no peer certificate",
                    )));
                }
            }
        };

        let config = rustls::ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(min_tls_version.to_rustls())
            .expect("cipher suites or kx groups mismatch with TLS version")
            .with_client_cert_verifier(verifier)
            .with_single_cert(local_certs, private_key)
            .map_err(|err| {
                TlsError::InvalidLocalCertificate(io::Error::new(
                    ErrorKind::InvalidData,
                    err.to_string(),
                ))
            })?;

        Ok(Self {
            config: std::sync::Arc::new(config),
        })
    }

    pub(crate) async fn handle_connection(
        &mut self,
        socket: TcpStream,
    ) -> Result<PhysLayer, String> {
        let connector = tokio_rustls::TlsAcceptor::from(self.config.clone());
        match connector.accept(socket).await {
            Err(err) => Err(format!("failed to establish TLS session: {}", err)),
            Ok(stream) => Ok(PhysLayer::Tls(Box::new(tokio_rustls::TlsStream::from(
                stream,
            )))),
        }
    }
}

struct CaChainClientCertVerifier {
    inner: Arc<dyn rustls::server::ClientCertVerifier>,
    dns_name: webpki::DnsName,
}

impl CaChainClientCertVerifier {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        roots: rustls::RootCertStore,
        dns_name: webpki::DnsName,
    ) -> Arc<dyn rustls::server::ClientCertVerifier> {
        let inner = AllowAnyAuthenticatedClient::new(roots);
        Arc::new(CaChainClientCertVerifier { inner, dns_name })
    }
}

impl rustls::server::ClientCertVerifier for CaChainClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        // Client must authenticate itself, so we better offer the authentication!
        true
    }

    fn client_auth_mandatory(&self) -> Option<bool> {
        // Client must authenticate itself
        Some(true)
    }

    fn client_auth_root_subjects(&self) -> Option<rustls::DistinguishedNames> {
        self.inner.client_auth_root_subjects()
    }

    fn verify_client_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        now: std::time::SystemTime,
    ) -> Result<rustls::server::ClientCertVerified, rustls::Error> {
        self.inner
            .verify_client_cert(end_entity, intermediates, now)?;

        // Check DNS name
        let cert = webpki::EndEntityCert::try_from(end_entity.0.as_ref())
            .map_err(|_| rustls::Error::InvalidCertificateEncoding)?;
        cert.verify_is_valid_for_dns_name(self.dns_name.as_ref())
            .map_err(|_| {
                rustls::Error::InvalidCertificateData(
                    "client certificate is not valid for provided name".to_string(),
                )
            })
            .map(|_| rustls::server::ClientCertVerified::assertion())
    }
}

struct SelfSignedCertificateClientCertVerifier {
    cert: rustls::Certificate,
}

impl SelfSignedCertificateClientCertVerifier {
    #[allow(clippy::new_ret_no_self)]
    fn new(cert: rustls::Certificate) -> Arc<dyn rustls::server::ClientCertVerifier> {
        Arc::new(SelfSignedCertificateClientCertVerifier { cert })
    }
}

impl rustls::server::ClientCertVerifier for SelfSignedCertificateClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        // Client must authenticate itself, so we better offer the authentication!
        true
    }

    fn client_auth_mandatory(&self) -> Option<bool> {
        // Client must authenticate itself
        Some(true)
    }

    fn client_auth_root_subjects(&self) -> Option<rustls::DistinguishedNames> {
        // Let rustls extract the subjects
        let mut store = rustls::RootCertStore::empty();
        let _ = store.add(&self.cert);
        Some(store.subjects())
    }

    fn verify_client_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        now: std::time::SystemTime,
    ) -> Result<rustls::server::ClientCertVerified, rustls::Error> {
        // Check that no intermediate certificates are present
        if !intermediates.is_empty() {
            return Err(rustls::Error::General(format!(
                "client sent {} intermediate certificates, expected none",
                intermediates.len()
            )));
        }

        // Check that presented certificate matches byte-for-byte the expected certificate
        if end_entity != &self.cert {
            return Err(rustls::Error::InvalidCertificateData(
                "client certificate doesn't match the expected self-signed certificate".to_string(),
            ));
        }

        // Check that the certificate is still valid
        let parsed_cert = rasn::x509::Certificate::parse(&end_entity.0).map_err(|err| {
            rustls::Error::InvalidCertificateData(format!(
                "unable to parse cert with rasn: {:?}",
                err
            ))
        })?;

        let now = now
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| rustls::Error::FailedToGetCurrentTime)?;
        let now = rasn::types::UtcTime::from_seconds_since_epoch(now.as_secs());

        if !parsed_cert.tbs_certificate.value.validity.is_valid(now) {
            return Err(rustls::Error::InvalidCertificateData(
                "self-signed certificate is currently not valid".to_string(),
            ));
        }

        // We do not validate DNS name. Providing the exact same certificate is sufficient.

        Ok(rustls::server::ClientCertVerified::assertion())
    }
}
