use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::path::Path;
use std::sync::Arc;

use tokio_rustls::rustls;
use tokio_rustls::rustls::server::AllowAnyAuthenticatedClient;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::client::ClientTask;
use crate::tcp::tls::{
    load_certs, load_private_key, CertificateMode, MinTlsVersion, TlsClientConfig, TlsError,
};
use crate::tcp::{ClientState, ConnectOptions, EndpointList, PostConnectionHandler};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, Session};
use rx509;
use tokio::net::TcpStream;

use tracing::Instrument;

/// Spawn a TLS client task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[allow(clippy::too_many_arguments)]
pub fn spawn_outstation_tls_client(
    link_error_mode: LinkErrorMode,
    endpoints: EndpointList,
    connect_strategy: ConnectStrategy,
    connect_options: ConnectOptions,
    config: OutstationConfig,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    listener: Box<dyn Listener<ClientState>>,
    tls_config: TlsClientConfig,
) -> OutstationHandle {
    let main_addr = endpoints.main_addr().to_string();
    let (task, handle) = OutstationTask::create(
        Enabled::No,
        link_error_mode,
        config,
        application,
        information,
        control_handler,
    );
    let session = Session::outstation(task);
    let mut client = ClientTask::new(
        session,
        endpoints,
        connect_strategy,
        connect_options,
        PostConnectionHandler::Tls(tls_config),
        listener,
    );

    let future = async move {
        client
            .run()
            .instrument(tracing::info_span!("dnp3-outstation-tls-client", "endpoint" = ?main_addr))
            .await;
    };
    tokio::spawn(future);
    handle
}

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
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
        certificate_mode: CertificateMode,
    ) -> Result<Self, TlsError> {
        let mut peer_certs = load_certs(peer_cert_path, false)?;
        let local_certs = load_certs(local_cert_path, true)?;
        let private_key = load_private_key(private_key_path, password)?;

        let verifier: Arc<dyn rustls::server::ClientCertVerifier> = match certificate_mode {
            CertificateMode::AuthorityBased => {
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

                // Check that the DNS name is at least valid
                rustls::ServerName::try_from(name).map_err(|_| TlsError::InvalidDnsName)?;

                Arc::new(CaChainClientCertVerifier::new(roots, name.to_string()))
            }
            CertificateMode::SelfSigned => {
                if let Some(peer_cert) = peer_certs.pop() {
                    if !peer_certs.is_empty() {
                        return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                            ErrorKind::InvalidData,
                            "more than one peer certificate in self-signed mode",
                        )));
                    }

                    Arc::new(SelfSignedCertificateClientCertVerifier::new(peer_cert))
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
            .map_err(|err| {
                TlsError::Other(io::Error::new(ErrorKind::InvalidData, err.to_string()))
            })?
            .with_client_cert_verifier(verifier)
            .with_single_cert(local_certs, private_key)
            .map_err(|err| {
                TlsError::InvalidLocalCertificate(io::Error::new(
                    ErrorKind::InvalidData,
                    err.to_string(),
                ))
            })?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    pub(crate) async fn handle_connection(
        &mut self,
        socket: TcpStream,
    ) -> Result<PhysLayer, String> {
        let connector = tokio_rustls::TlsAcceptor::from(self.config.clone());
        match connector.accept(socket).await {
            Err(err) => Err(format!("failed to establish TLS session: {err}")),
            Ok(stream) => Ok(PhysLayer::Tls(Box::new(tokio_rustls::TlsStream::from(
                stream,
            )))),
        }
    }
}

struct CaChainClientCertVerifier {
    inner: Arc<dyn rustls::server::ClientCertVerifier>,
    peer_name: String,
}

impl CaChainClientCertVerifier {
    fn new(roots: rustls::RootCertStore, peer_name: String) -> Self {
        let inner = AllowAnyAuthenticatedClient::new(roots);
        Self { inner, peer_name }
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

        // Check DNS name (including in the Common Name)
        super::verify_dns_name(end_entity, &self.peer_name)?;

        Ok(rustls::server::ClientCertVerified::assertion())
    }
}

struct SelfSignedCertificateClientCertVerifier {
    cert: rustls::Certificate,
}

impl SelfSignedCertificateClientCertVerifier {
    fn new(cert: rustls::Certificate) -> Self {
        Self { cert }
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
        #[allow(deprecated)]
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

        Ok(rustls::server::ClientCertVerified::assertion())
    }
}
