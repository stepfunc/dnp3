use std::convert::TryFrom;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::tls::{CertificateMode, MinTlsVersion, TlsError};
use crate::tcp::{wire_master_client, ClientState, ConnectOptions};
use crate::tcp::{EndpointList, PostConnectionHandler};
use crate::util::phys::PhysLayer;

use tokio::net::TcpStream;
use tokio_rustls::rustls;
use tracing::Instrument;

/// TLS configuration for a client
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
    /// Legacy method for creating a client TLS configuration
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

        let config = match certificate_mode {
            CertificateMode::AuthorityBased => sfio_rustls_config::client::authority(
                min_tls_version.into(),
                sfio_rustls_config::NameVerifier::equal_to(name.to_string()),
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
            )?,
            CertificateMode::SelfSigned => sfio_rustls_config::client::self_signed(
                min_tls_version.into(),
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
            )?,
        };

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
