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
        let config = match certificate_mode {
            CertificateMode::AuthorityBased => sfio_rustls_config::server::authority(
                min_tls_version.into(),
                super::dangerous::client_verifier(name),
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
            )?,
            CertificateMode::SelfSigned => sfio_rustls_config::server::self_signed(
                min_tls_version.into(),
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
            )?,
        };

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
