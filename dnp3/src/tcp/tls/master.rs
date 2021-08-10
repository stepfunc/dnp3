use std::future::Future;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
use std::path::Path;

use tokio_rustls::{rustls, webpki};
use tracing::Instrument;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::tls::{load_certs, load_private_key, TlsError};
use crate::tcp::EndpointList;
use crate::tcp::{ClientState, MasterTask, MasterTaskConnectionHandler};
use crate::tokio::net::TcpStream;
use crate::util::phys::PhysLayer;

/// TLS configuration
pub struct TlsClientConfig {
    dns_name: webpki::DnsName,
    config: std::sync::Arc<rustls::ClientConfig>,
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
        allow_tls_1_2: bool,
    ) -> Result<Self, TlsError> {
        let peer_certs = load_certs(peer_cert_path, false)?;
        let local_certs = load_certs(local_cert_path, true)?;
        let private_key = load_private_key(private_key_path)?;

        let mut config = rustls::ClientConfig::new();

        // Add peer certificates
        for cert in &peer_certs {
            config.root_store.add(cert).map_err(|err| {
                TlsError::InvalidPeerCertificate(io::Error::new(
                    ErrorKind::InvalidData,
                    err.to_string(),
                ))
            })?;
        }

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
        config.versions = vec![rustls::ProtocolVersion::TLSv1_3];
        if allow_tls_1_2 {
            config.versions.push(rustls::ProtocolVersion::TLSv1_2);
        }

        let dns_name = webpki::DnsNameRef::try_from_ascii_str(name)
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
