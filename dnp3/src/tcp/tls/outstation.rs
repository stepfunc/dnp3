use sfio_rustls_config::ClientNameVerification;
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
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, Session};
use tokio::net::TcpStream;
use tokio_rustls::rustls::pki_types::ServerName;

use crate::link::reader::LinkModes;
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
        LinkModes::stream(link_error_mode),
        config,
        PhysAddr::None,
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
    /// Legacy method of creating a TLS server configuration
    #[deprecated(
        since = "1.4.1",
        note = "Please use `full_pki` or `self_signed` instead"
    )]
    pub fn new(
        client_subject_name: &str,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
        certificate_mode: CertificateMode,
    ) -> Result<Self, TlsError> {
        match certificate_mode {
            CertificateMode::AuthorityBased => Self::full_pki(
                Some(client_subject_name.to_string()),
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
                min_tls_version,
            ),
            CertificateMode::SelfSigned => Self::self_signed(
                peer_cert_path,
                local_cert_path,
                private_key_path,
                password,
                min_tls_version,
            ),
        }
    }

    /// Create a TLS server configuration that expects a full PKI with an authority, and possibly
    /// intermediate CA certificates.
    ///
    /// If `client_subject_name` is specified, than the server will verify name is present in the
    /// SAN extension or in the Common Name of the client certificate.
    ///
    /// If `client_subject_name` is set to None, then no client name validate is performed, and
    /// any authenticated client is allowed.
    pub fn full_pki(
        client_subject_name: Option<String>,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
    ) -> Result<Self, TlsError> {
        let name_verification = match client_subject_name {
            None => ClientNameVerification::None,
            Some(name) => {
                let name: ServerName<'static> = name.try_into()?;
                ClientNameVerification::SanOrCommonName(name)
            }
        };

        let config = sfio_rustls_config::server::authority(
            min_tls_version.into(),
            name_verification,
            peer_cert_path,
            local_cert_path,
            private_key_path,
            password,
        )?;

        Ok(Self {
            config: Arc::new(config),
        })
    }

    /// Create a TLS server configuration that expects the client to present a single certificate.
    ///
    /// In lieu of performing client subject name validation, the server validates:
    ///
    /// 1) That the client presents a single certificate
    /// 2) That the certificate is a byte-for-byte match with the one loaded in `peer_cert_path`.
    /// 3) That the certificate's Validity (not before / not after) is currently valid.
    ///
    pub fn self_signed(
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
    ) -> Result<Self, TlsError> {
        let config = sfio_rustls_config::server::self_signed(
            min_tls_version.into(),
            peer_cert_path,
            local_cert_path,
            private_key_path,
            password,
        )?;

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
