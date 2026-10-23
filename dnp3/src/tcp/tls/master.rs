// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use sfio_rustls_config::ServerNameVerification;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::Arc;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::master::{MasterChannel, MasterChannelConfig};
use crate::tcp::tls::{CertificateMode, HandshakeError, MinTlsVersion, TlsError};
use crate::tcp::{
    spawn_master_client, ClientConnectionHandler, ClientState, ConnectOptions, SimpleConnectHandler,
};
use crate::tcp::{EndpointList, PostConnectionHandler};
use crate::util::phys::{PhysLayer, Socket};

use tokio_rustls::rustls;
use tokio_rustls::rustls::pki_types::{IpAddr, ServerName};

/// TLS configuration for a client
pub struct TlsClientConfig {
    // server name used in SNI - if and only if it's a DNS name, does nothing for IP
    server_name: ServerName<'static>,
    config: Arc<rustls::ClientConfig>,
}

impl std::fmt::Debug for TlsClientConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // the rustls configuration holds key material, so only the server name is shown
        f.debug_struct("TlsClientConfig")
            .field("server_name", &self.server_name)
            .finish_non_exhaustive()
    }
}

/// Spawn a task onto the `Tokio` runtime. The task runs until the returned handle, and any
/// `AssociationHandle` created from it is dropped.
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
    let handler = SimpleConnectHandler::create(endpoints, connect_options, connect_strategy);
    spawn_master_tls_client_3(link_error_mode, config, handler, listener, tls_config)
}

/// Just like [spawn_master_tls_client_2], but this variant provides fine-grained
/// control over the connection management using a user-defined implementation
/// of [`ClientConnectionHandler`]
pub fn spawn_master_tls_client_3(
    link_error_mode: LinkErrorMode,
    config: MasterChannelConfig,
    connection_handler: Box<dyn ClientConnectionHandler>,
    listener: Box<dyn Listener<ClientState>>,
    tls_config: TlsClientConfig,
) -> MasterChannel {
    spawn_master_client(
        link_error_mode,
        config,
        connection_handler,
        PostConnectionHandler::Tls(tls_config),
        listener,
        |name| tracing::info_span!("dnp3-master-tls-client", "endpoint" = ?name),
    )
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

    pub(crate) async fn handle_connection<S: Socket>(
        &self,
        socket: S,
        endpoint: &SocketAddr,
    ) -> Result<PhysLayer, HandshakeError> {
        let stream = self
            .connect(socket)
            .await
            .inspect_err(|err| err.log(*endpoint))?;
        Ok(S::into_tls_phys(tokio_rustls::TlsStream::from(stream)))
    }

    /// Run the client side of the TLS handshake over a connected socket
    pub(super) async fn connect<S>(
        &self,
        socket: S,
    ) -> Result<tokio_rustls::client::TlsStream<S>, HandshakeError>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let connector = tokio_rustls::TlsConnector::from(self.config.clone());
        // a server that stalls the handshake would otherwise stop the client retrying or moving on to
        // its other endpoints
        super::handshake(connector.connect(self.server_name.clone(), socket)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcp::tls::TLS_HANDSHAKE_TIMEOUT;
    use crate::util::test_support::tls::client_config;
    use crate::util::test_support::watchdog;
    use tokio::net::TcpStream;

    #[tokio::test(start_paused = true)]
    async fn tls_handshake_with_a_server_that_never_answers_times_out() {
        watchdog(async {
            let config = client_config(MinTlsVersion::V12);

            // a server that accepts the connection and never answers the ClientHello
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = listener.local_addr().unwrap();
            let socket = TcpStream::connect(endpoint).await.unwrap();
            let _server = listener.accept().await.unwrap();

            let start = tokio::time::Instant::now();
            let result = config.handle_connection(socket, &endpoint).await;
            assert!(matches!(result, Err(HandshakeError::Timeout)));
            assert_eq!(start.elapsed(), TLS_HANDSHAKE_TIMEOUT);
        })
        .await;
    }
}
