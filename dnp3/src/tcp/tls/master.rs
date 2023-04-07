use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::tls::{CertificateMode, MinTlsVersion, NameVerifier, TlsError};
use crate::tcp::{wire_master_client, ClientState, ConnectOptions};
use crate::tcp::{EndpointList, PostConnectionHandler};
use crate::util::phys::PhysLayer;

use rx509;
use tokio::net::TcpStream;
use tokio_rustls::{rustls, webpki};
use tracing::Instrument;

/// TLS configuration
pub struct TlsClientConfig {
    dns_name: rustls::ServerName,
    config: Arc<rustls::ClientConfig>,
}

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
pub fn spawn_master_tls_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
    tls_config: TlsClientConfig,
) -> MasterChannel {
    spawn_master_tls_client_2(
        link_error_mode,
        config,
        endpoints,
        connect_strategy,
        ConnectOptions::default(),
        listener,
        tls_config,
    )
}

/// Just like [spawn_master_tls_client], but this variant was added later to also accept and
/// apply [ConnectOptions]
pub fn spawn_master_tls_client_2(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    connect_options: ConnectOptions,
    listener: Box<dyn Listener<ClientState>>,
    tls_config: TlsClientConfig,
) -> MasterChannel {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = wire_master_client(
        link_error_mode,
        endpoints,
        config,
        connect_strategy,
        connect_options,
        PostConnectionHandler::Tls(tls_config),
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("dnp3-master-tls-client", "endpoint" = ?main_addr))
            .await;
    };
    tokio::spawn(future);
    handle
}

impl TlsClientConfig {
    /// Create a TLS master config
    pub fn new(
        name: &str,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
        certificate_mode: CertificateMode,
    ) -> Result<Self, TlsError> {
        let dns_name = rustls::ServerName::try_from(name)?;

        let peer_certs: Vec<rustls::Certificate> = {
            let data = std::fs::read(peer_cert_path)?;
            let certs = pem_util::read_certificates_from_pem(data)?;
            certs.into_iter().map(rustls::Certificate).collect()
        };

        let local_certs: Vec<rustls::Certificate> = {
            let data = std::fs::read(local_cert_path)?;
            let certs = pem_util::read_certificates_from_pem(data)?;
            certs.into_iter().map(rustls::Certificate).collect()
        };

        let private_key: rustls::PrivateKey = {
            let key_bytes = std::fs::read(private_key_path)?;
            let key = match password {
                Some(x) => pem_util::PrivateKey::decrypt_from_pem(key_bytes, x)?,
                None => pem_util::PrivateKey::read_from_pem(key_bytes)?,
            };
            rustls::PrivateKey(key.bytes().to_vec())
        };

        fn trust_anchors(
            certs: Vec<rustls::Certificate>,
        ) -> Result<Vec<OwnedTrustAnchor>, webpki::Error> {
            certs
                .iter()
                .map(|x| OwnedTrustAnchor::try_from_cert_der(x.0.as_slice()))
                .collect()
        }

        let verifier: Arc<dyn rustls::client::ServerCertVerifier> = match certificate_mode {
            CertificateMode::AuthorityBased => {
                // Build trust roots
                let root: Vec<OwnedTrustAnchor> = trust_anchors(peer_certs)?;
                let name_verifier = NameVerifier::try_create(name.to_string())?;
                Arc::new(CommonNameServerCertVerifier::new(root, name_verifier))
            }
            CertificateMode::SelfSigned => {
                let peer_cert = match peer_certs.as_slice() {
                    &[] => {
                        return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                            ErrorKind::InvalidData,
                            "no peer certificate",
                        )))
                    }
                    [single] => single.clone(),
                    _ => {
                        return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                            ErrorKind::InvalidData,
                            "more than one peer certificate in self-signed mode",
                        )))
                    }
                };
                Arc::new(SelfSignedCertificateServerCertVerifier::new(peer_cert))
            }
        };

        let config = rustls::ClientConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(min_tls_version.to_rustls())?
            .with_custom_certificate_verifier(verifier)
            .with_single_cert(local_certs, private_key)?;

        Ok(Self {
            config: Arc::new(config),
            dns_name,
        })
    }

    pub(crate) async fn handle_connection(
        &mut self,
        socket: TcpStream,
        endpoint: &SocketAddr,
    ) -> Option<PhysLayer> {
        let connector = tokio_rustls::TlsConnector::from(self.config.clone());
        match connector.connect(self.dns_name.clone(), socket).await {
            Err(err) => {
                tracing::warn!("failed to establish TLS session with {endpoint}: {err}");
                None
            }
            Ok(stream) => Some(PhysLayer::Tls(Box::new(tokio_rustls::TlsStream::from(
                stream,
            )))),
        }
    }
}

struct CommonNameServerCertVerifier {
    roots: Vec<OwnedTrustAnchor>,
    verifier: NameVerifier,
}

impl CommonNameServerCertVerifier {
    fn new(roots: Vec<OwnedTrustAnchor>, verifier: NameVerifier) -> Self {
        Self { roots, verifier }
    }
}

impl rustls::client::ServerCertVerifier for CommonNameServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        // Note: this code is taken from `WebPkiVerifier` in the `verifier` module of `rustls`

        // Verify trust chain using webpki
        let (cert, chain, trustroots) = prepare(end_entity, intermediates, &self.roots)?;
        let webpki_now =
            webpki::Time::try_from(now).map_err(|_| rustls::Error::FailedToGetCurrentTime)?;

        cert.verify_is_valid_tls_server_cert(
            SUPPORTED_SIG_ALGS,
            &webpki::TlsServerTrustAnchors(&trustroots),
            &chain,
            webpki_now,
        )
        .map_err(super::pki_error)
        .map(|_| cert)?;

        // Check DNS name (including in the Common Name)
        self.verifier.verify(end_entity)?;

        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];

static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::ED25519,
    &webpki::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

// TODO: if `rustls::OwnedTrustAnchor::to_trust_anchor` was public,
// we wouldn't need to duplicate this.
#[derive(Debug, Clone)]
struct OwnedTrustAnchor {
    subject: Vec<u8>,
    spki: Vec<u8>,
    name_constraints: Option<Vec<u8>>,
}

impl OwnedTrustAnchor {
    /// Get a `webpki::TrustAnchor` by borrowing the owned elements.
    fn to_trust_anchor(&self) -> webpki::TrustAnchor {
        webpki::TrustAnchor {
            subject: &self.subject,
            spki: &self.spki,
            name_constraints: self.name_constraints.as_deref(),
        }
    }

    fn try_from_cert_der(cert_der: &[u8]) -> Result<Self, webpki::Error> {
        let trust_anchor = webpki::TrustAnchor::try_from_cert_der(cert_der)?;

        Ok(Self {
            subject: trust_anchor.subject.to_owned(),
            spki: trust_anchor.spki.to_owned(),
            name_constraints: trust_anchor.name_constraints.map(|x| x.to_owned()),
        })
    }
}

type CertChainAndRoots<'a, 'b> = (
    webpki::EndEntityCert<'a>,
    Vec<&'a [u8]>,
    Vec<webpki::TrustAnchor<'b>>,
);

fn prepare<'a, 'b>(
    end_entity: &'a rustls::Certificate,
    intermediates: &'a [rustls::Certificate],
    roots: &'b [OwnedTrustAnchor],
) -> Result<CertChainAndRoots<'a, 'b>, rustls::Error> {
    // EE cert must appear first.
    let cert = webpki::EndEntityCert::try_from(end_entity.0.as_ref()).map_err(super::pki_error)?;

    let intermediates: Vec<&'a [u8]> = intermediates.iter().map(|cert| cert.0.as_ref()).collect();

    let trustroots: Vec<webpki::TrustAnchor> = roots
        .iter()
        .map(OwnedTrustAnchor::to_trust_anchor)
        .collect();

    Ok((cert, intermediates, trustroots))
}

struct SelfSignedCertificateServerCertVerifier {
    cert: rustls::Certificate,
}

impl SelfSignedCertificateServerCertVerifier {
    fn new(cert: rustls::Certificate) -> Self {
        Self { cert }
    }
}

impl rustls::client::ServerCertVerifier for SelfSignedCertificateServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::Certificate,
        intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
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

        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
