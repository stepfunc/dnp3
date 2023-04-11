use std::path::Path;
use std::sync::Arc;

use tokio_rustls::rustls;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::outstation::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::client::ClientTask;
use crate::tcp::tls::{CertificateMode, MinTlsVersion, TlsClientConfig, TlsError};
use crate::tcp::{ClientState, ConnectOptions, EndpointList, PostConnectionHandler};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, Session};
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

/// TLS configuration for a server
pub struct TlsServerConfig {
    config: Arc<rustls::ServerConfig>,
}

impl TlsServerConfig {
    /// Create a TLS server config.
    ///
    /// The name field is what gets verified from the peer certificate. Name verification
    /// can be disabled by first calling [crate::tcp::tls::dangerous::enable_peer_name_wildcards]
    /// and then passing "*" to this function.
    pub fn new(
        name: &str,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
        certificate_mode: CertificateMode,
    ) -> Result<Self, TlsError> {
        let peer_certs: Vec<rustls::Certificate> = {
            let data = std::fs::read(peer_cert_path)?;
            let certs = sfio_pem_util::read_certificates(data)?;
            certs.into_iter().map(rustls::Certificate).collect()
        };

        let local_certs: Vec<rustls::Certificate> = {
            let data = std::fs::read(local_cert_path)?;
            let certs = sfio_pem_util::read_certificates(data)?;
            certs.into_iter().map(rustls::Certificate).collect()
        };

        let private_key: rustls::PrivateKey = {
            let key_bytes = std::fs::read(private_key_path)?;
            let key = match password {
                Some(x) => sfio_pem_util::PrivateKey::decrypt_from_pem(key_bytes, x)?,
                None => sfio_pem_util::PrivateKey::read_from_pem(key_bytes)?,
            };
            rustls::PrivateKey(key.bytes().to_vec())
        };

        let verifier: Arc<dyn rustls::server::ClientCertVerifier> = match certificate_mode {
            CertificateMode::AuthorityBased => {
                // Build root certificate store
                let mut roots = rustls::RootCertStore::empty();
                for cert in &peer_certs {
                    roots.add(cert)?;
                }
                let verifier = sfio_rustls_util::ClientCertVerifier::new(
                    roots,
                    super::dangerous::verifier(name),
                );
                Arc::new(verifier)
            }
            CertificateMode::SelfSigned => {
                let cert = super::expect_single_peer_cert(peer_certs)?;
                let verifier = sfio_rustls_util::SelfSignedVerifier::create(cert)?;
                Arc::new(verifier)
            }
        };

        let config = rustls::ServerConfig::builder()
            .with_safe_default_cipher_suites()
            .with_safe_default_kx_groups()
            .with_protocol_versions(min_tls_version.to_rustls())?
            .with_client_cert_verifier(verifier)
            .with_single_cert(local_certs, private_key)?;

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
