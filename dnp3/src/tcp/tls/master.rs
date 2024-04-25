use sfio_rustls_config::ServerNameVerification;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::master::{MasterChannel, MasterChannelConfig, MasterChannelType};
use crate::tcp::tls::{CertificateMode, MinTlsVersion, TlsError};
use crate::tcp::{wire_master_client, ClientState, ConnectOptions};
use crate::tcp::{EndpointList, PostConnectionHandler};
use crate::util::phys::PhysLayer;

use crate::link::reader::LinkModes;
use tokio::net::TcpStream;
use tokio_rustls::rustls;
use tokio_rustls::rustls::pki_types::{IpAddr, ServerName};
use tracing::Instrument;

/// TLS configuration for a client
pub struct TlsClientConfig {
    // server name used in SNI - if and only if it's a DNS name, does nothing for IP
    server_name: ServerName<'static>,
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
        LinkModes::stream(link_error_mode),
        MasterChannelType::Stream,
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
    #[deprecated(
        since = "1.4.1",
        note = "Please use `full_pki` or `self_signed` instead"
    )]
    pub fn new(
        server_name: &str,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
        certificate_mode: CertificateMode,
    ) -> Result<Self, TlsError> {
        match certificate_mode {
            CertificateMode::AuthorityBased => Self::full_pki(
                Some(server_name.to_string()),
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

    /// Create a TLS client configuration that expects a full PKI with an authority, and possibly
    /// intermediate CA certificates.
    ///
    /// If `server_subject_name` is specified, than the client will verify that the name is present in the
    /// SAN extension or in the Common Name of the client certificate.
    ///
    /// If `server_subject_name` is set to None, then no server name validation is performed, and
    /// any authenticated server is allowed.
    pub fn full_pki(
        server_subject_name: Option<String>,
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        password: Option<&str>,
        min_tls_version: MinTlsVersion,
    ) -> Result<Self, TlsError> {
        let (name_verifier, server_name) = match server_subject_name {
            None => (
                ServerNameVerification::DisableNameVerification,
                ServerName::IpAddress(IpAddr::V4(Ipv4Addr::UNSPECIFIED.into())),
            ),
            Some(x) => {
                let server_name: ServerName<'static> = ServerName::try_from(x)?;
                (ServerNameVerification::SanOrCommonName, server_name)
            }
        };

        let config = sfio_rustls_config::client::authority(
            min_tls_version.into(),
            name_verifier,
            peer_cert_path,
            local_cert_path,
            private_key_path,
            password,
        )?;

        Ok(Self {
            server_name,
            config: Arc::new(config),
        })
    }

    /// Create a TLS client configuration that expects the client to present a single certificate.
    ///
    /// In lieu of performing server subject name validation, the client validates:
    ///
    /// 1) That the server presents a single certificate
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
        let config = sfio_rustls_config::client::self_signed(
            min_tls_version.into(),
            peer_cert_path,
            local_cert_path,
            private_key_path,
            password,
        )?;

        Ok(Self {
            //  it doesn't matter what we put here, it just needs to be an IP so that the client won't send an SNI extension
            server_name: ServerName::IpAddress(IpAddr::V4(Ipv4Addr::UNSPECIFIED.into())),
            config: Arc::new(config),
        })
    }

    pub(crate) async fn handle_connection(
        &mut self,
        socket: TcpStream,
        endpoint: &SocketAddr,
    ) -> Option<PhysLayer> {
        let connector = tokio_rustls::TlsConnector::from(self.config.clone());
        match connector.connect(self.server_name.clone(), socket).await {
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
