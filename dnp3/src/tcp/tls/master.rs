use std::future::Future;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use tokio_rustls::{rustls, webpki};
use tracing::Instrument;

use super::rasn;
use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::tls::{load_certs, load_private_key, CertificateMode, MinTlsVersion, TlsError};
use crate::tcp::EndpointList;
use crate::tcp::{ClientState, MasterTask, MasterTaskConnectionHandler};
use crate::tokio::net::TcpStream;
use crate::util::phys::PhysLayer;

/// TLS configuration
pub struct TlsClientConfig {
    dns_name: webpki::DNSName,
    config: Arc<rustls::ClientConfig>,
}

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it, are dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// It is preferable to use this method instead of `create(..)` when using `[tokio::main]`.
pub fn spawn_master_tls_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    tls_config: TlsClientConfig,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> MasterChannel {
    let (future, handle) = create_master_tls_client(
        link_error_mode,
        config,
        endpoints,
        tls_config,
        connect_strategy,
        listener,
    );
    crate::tokio::spawn(future);
    handle
}

/// Create a Future, which can be spawned onto a runtime, along with a controlling handle.
///
/// Once spawned or otherwise executed using the `run` method, the task runs until the handle
/// and any `AssociationHandle` created from it are dropped.
///
/// **Note**: This function is required instead of `spawn` when using a runtime to directly spawn
/// tasks instead of within the context of a runtime, e.g. in applications that cannot use
/// `[tokio::main]` such as C language bindings.
pub fn create_master_tls_client(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    endpoints: EndpointList,
    tls_config: TlsClientConfig,
    connect_strategy: ConnectStrategy,
    listener: Box<dyn Listener<ClientState>>,
) -> (impl Future<Output = ()> + 'static, MasterChannel) {
    let main_addr = endpoints.main_addr().to_string();
    let (mut task, handle) = MasterTask::new(
        link_error_mode,
        endpoints,
        config,
        connect_strategy,
        MasterTaskConnectionHandler::Tls(tls_config),
        listener,
    );
    let future = async move {
        task.run()
            .instrument(tracing::info_span!("DNP3-Master-TLS", "endpoint" = ?main_addr))
            .await;
    };
    (future, handle)
}

impl TlsClientConfig {
    /// Create a TLS master config
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

        let mut config = rustls::ClientConfig::new();

        // Set local cert chain
        config
            .set_single_client_cert(local_certs, private_key)
            .map_err(|err| {
                TlsError::InvalidLocalCertificate(io::Error::new(
                    ErrorKind::InvalidData,
                    err.to_string(),
                ))
            })?;

        // Set allowed TLS versions
        config.versions = min_tls_version.to_vec();

        match certificate_mode {
            CertificateMode::TrustChain => {
                // Add peer certificates
                // The default WebPKIVerifier will validate the presented
                // cert chain against these.
                if certificate_mode == CertificateMode::TrustChain {
                    for cert in &peer_certs {
                        config.root_store.add(cert).map_err(|err| {
                            TlsError::InvalidPeerCertificate(io::Error::new(
                                ErrorKind::InvalidData,
                                err.to_string(),
                            ))
                        })?;
                    }
                }
            }
            CertificateMode::SelfSignedCertificate => {
                // Set the custom certificate verifier
                if certificate_mode == CertificateMode::SelfSignedCertificate {
                    if let Some(peer_cert) = peer_certs.pop() {
                        if !peer_certs.is_empty() {
                            return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                                ErrorKind::InvalidData,
                                "more than one peer certificate in self-signed mode",
                            )));
                        }

                        config.dangerous().set_certificate_verifier(Arc::new(
                            SelfSignedCertificateServerCertVerifier::new(peer_cert),
                        ));
                    } else {
                        return Err(TlsError::InvalidPeerCertificate(io::Error::new(
                            ErrorKind::InvalidData,
                            "no peer certificate",
                        )));
                    }
                }
            }
        }

        let dns_name = webpki::DNSNameRef::try_from_ascii_str(name)
            .map_err(|_| TlsError::InvalidDnsName)?
            .to_owned();

        Ok(Self {
            config: std::sync::Arc::new(config),
            dns_name,
        })
    }

    pub(crate) async fn handle_connection(
        &mut self,
        socket: TcpStream,
        endpoint: &SocketAddr,
    ) -> Result<PhysLayer, String> {
        let connector = tokio_rustls::TlsConnector::from(self.config.clone());
        match connector.connect(self.dns_name.as_ref(), socket).await {
            Err(err) => Err(format!(
                "failed to establish TLS session with {}: {}",
                endpoint, err
            )),
            Ok(stream) => Ok(PhysLayer::Tls(Box::new(tokio_rustls::TlsStream::from(
                stream,
            )))),
        }
    }
}

struct SelfSignedCertificateServerCertVerifier {
    cert: rustls::Certificate,
}

impl SelfSignedCertificateServerCertVerifier {
    fn new(cert: rustls::Certificate) -> Self {
        Self { cert }
    }
}

impl rustls::ServerCertVerifier for SelfSignedCertificateServerCertVerifier {
    fn verify_server_cert(
        &self,
        _roots: &rustls::RootCertStore,
        presented_certs: &[rustls::Certificate],
        _dns_name: webpki::DNSNameRef<'_>,
        _ocsp_response: &[u8],
    ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        let now = chrono::offset::Utc::now();

        // Check that only 1 certificate is presented
        if presented_certs.len() != 1 {
            return Err(rustls::TLSError::General(format!(
                "server sent {} certificates, expected one",
                presented_certs.len()
            )));
        }

        // Check that presented certificate matches byte-for-byte the expected certificate
        if presented_certs[0] != self.cert {
            return Err(rustls::TLSError::General(
                "server certificate doesn't match the expected self-signed certificate".to_string(),
            ));
        }

        // Check that the certificate is still valid
        let parsed_cert = rasn::x509::Certificate::parse(&presented_certs[0].0).map_err(|err| {
            rustls::TLSError::General(format!("unable to parse cert with rasn: {:?}", err))
        })?;
        parsed_cert
            .tbs_certificate
            .value
            .validity
            .is_valid(now.into());

        // We do not validate DNS name. Providing the exact same certificate is sufficient.

        Ok(rustls::ServerCertVerified::assertion())
    }
}
