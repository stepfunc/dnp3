use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::Arc;

use tokio_rustls::rustls::AllowAnyAuthenticatedClient;
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

                let dns_name = webpki::DNSNameRef::try_from_ascii_str(name)
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

        let mut config = rustls::ServerConfig::new(verifier);

        // Set local cert chain
        config
            .set_single_cert(local_certs, private_key)
            .map_err(|err| {
                TlsError::InvalidLocalCertificate(io::Error::new(
                    ErrorKind::InvalidData,
                    err.to_string(),
                ))
            })?;

        // Set allowed TLS versions
        config.versions = min_tls_version.to_vec();

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
    inner: Arc<dyn rustls::ClientCertVerifier>,
    dns_name: webpki::DNSName,
}

impl CaChainClientCertVerifier {
    #[allow(clippy::new_ret_no_self)]
    fn new(
        roots: rustls::RootCertStore,
        dns_name: webpki::DNSName,
    ) -> Arc<dyn rustls::ClientCertVerifier> {
        let inner = AllowAnyAuthenticatedClient::new(roots);
        Arc::new(CaChainClientCertVerifier { inner, dns_name })
    }
}

impl rustls::ClientCertVerifier for CaChainClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        // Client must authenticate itself, so we better offer the authentication!
        true
    }

    fn client_auth_mandatory(&self, _sni: Option<&webpki::DNSName>) -> Option<bool> {
        // Client must authenticate itself
        Some(true)
    }

    fn client_auth_root_subjects(
        &self,
        sni: Option<&webpki::DNSName>,
    ) -> Option<rustls::DistinguishedNames> {
        self.inner.client_auth_root_subjects(sni)
    }

    fn verify_client_cert(
        &self,
        presented_certs: &[rustls::Certificate],
        sni: Option<&webpki::DNSName>,
    ) -> Result<rustls::ClientCertVerified, rustls::TLSError> {
        self.inner.verify_client_cert(presented_certs, sni)?;

        // Check DNS name
        let cert = webpki::EndEntityCert::from(&presented_certs[0].0)
            .map_err(rustls::TLSError::WebPKIError)?;
        cert.verify_is_valid_for_dns_name(self.dns_name.as_ref())
            .map_err(|_| {
                rustls::TLSError::General(
                    "client certificate is not valid for provided name".to_string(),
                )
            })
            .map(|_| rustls::ClientCertVerified::assertion())
    }
}

struct SelfSignedCertificateClientCertVerifier {
    cert: rustls::Certificate,
}

impl SelfSignedCertificateClientCertVerifier {
    #[allow(clippy::new_ret_no_self)]
    fn new(cert: rustls::Certificate) -> Arc<dyn rustls::ClientCertVerifier> {
        Arc::new(SelfSignedCertificateClientCertVerifier { cert })
    }
}

impl rustls::ClientCertVerifier for SelfSignedCertificateClientCertVerifier {
    fn offer_client_auth(&self) -> bool {
        // Client must authenticate itself, so we better offer the authentication!
        true
    }

    fn client_auth_mandatory(&self, _sni: Option<&webpki::DNSName>) -> Option<bool> {
        // Client must authenticate itself
        Some(true)
    }

    fn client_auth_root_subjects(
        &self,
        _sni: Option<&webpki::DNSName>,
    ) -> Option<rustls::DistinguishedNames> {
        // Let rustls extract the subjects
        let mut store = rustls::RootCertStore::empty();
        let _ = store.add(&self.cert);
        Some(store.get_subjects())
    }

    fn verify_client_cert(
        &self,
        presented_certs: &[rustls::Certificate],
        _sni: Option<&webpki::DNSName>,
    ) -> Result<rustls::ClientCertVerified, rustls::TLSError> {
        // Check that only 1 certificate is presented
        if presented_certs.len() != 1 {
            return Err(rustls::TLSError::General(format!(
                "client sent {} certificates, expected one",
                presented_certs.len()
            )));
        }

        // Check that presented certificate matches byte-for-byte the expected certificate
        if presented_certs[0] != self.cert {
            return Err(rustls::TLSError::General(
                "client certificate doesn't match the expected self-signed certificate".to_string(),
            ));
        }

        // Check that the certificate is still valid
        let parsed_cert = rasn::x509::Certificate::parse(&presented_certs[0].0).map_err(|err| {
            rustls::TLSError::General(format!("unable to parse cert with rasn: {:?}", err))
        })?;
        let now = rasn::types::UtcTime::now()
            .map_err(|_| rustls::TLSError::General("unable to get the current time".to_string()))?;
        if !parsed_cert.tbs_certificate.value.validity.is_valid(now) {
            return Err(rustls::TLSError::General(
                "self-signed certificate is currently not valid".to_string(),
            ));
        }

        // We do not validate DNS name. Providing the exact same certificate is sufficient.

        Ok(rustls::ClientCertVerified::assertion())
    }
}
