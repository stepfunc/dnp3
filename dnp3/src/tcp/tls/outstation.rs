// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use sfio_rustls_config::ClientNameVerification;
use std::path::Path;
use std::sync::Arc;

use tokio_rustls::rustls;

use crate::app::{ConnectStrategy, Listener};
use crate::link::LinkErrorMode;
use crate::outstation::session::task::OutstationTask;
use crate::outstation::{
    ControlHandler, OutstationApplication, OutstationConfig, OutstationHandle,
    OutstationInformation,
};
use crate::tcp::tls::{CertificateMode, HandshakeError, MinTlsVersion, TlsClientConfig, TlsError};
use crate::tcp::{
    ClientConnectionHandler, ClientState, ConnectOptions, EndpointList, PostConnectionHandler,
    SimpleConnectHandler,
};
use crate::util::lifecycle::Enabled;
use crate::util::phys::{PhysAddr, PhysLayer, Socket};
use tokio_rustls::rustls::pki_types::ServerName;

use crate::link::reader::LinkModes;

/// Spawn a TLS client task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[expect(
    clippy::too_many_arguments,
    reason = "the public spawn function takes each part of its configuration separately"
)]
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
    let connect_handler =
        SimpleConnectHandler::create(endpoints, connect_options, connect_strategy);
    spawn_outstation_tls_client_2(
        link_error_mode,
        config,
        connect_handler,
        application,
        information,
        control_handler,
        listener,
        tls_config,
    )
}

/// Spawn a TLS client task onto the `Tokio` runtime. The task runs until the returned handle is dropped.
///
/// This function is similar to [`spawn_outstation_tls_client`] but provides fine-grained control over
/// connection management via a user implementation of the [`ClientConnectionHandler`] trait.
///
/// **Note**: This function may only be called from within the runtime itself, and panics otherwise.
/// Use Runtime::enter() if required.
#[expect(
    clippy::too_many_arguments,
    reason = "the public spawn function takes each part of its configuration separately"
)]
pub fn spawn_outstation_tls_client_2(
    link_error_mode: LinkErrorMode,
    config: OutstationConfig,
    connect_handler: Box<dyn ClientConnectionHandler>,
    application: Box<dyn OutstationApplication>,
    information: Box<dyn OutstationInformation>,
    control_handler: Box<dyn ControlHandler>,
    listener: Box<dyn Listener<ClientState>>,
    tls_config: TlsClientConfig,
) -> OutstationHandle {
    let (task, handle) = OutstationTask::create(
        Enabled::No,
        LinkModes::stream(link_error_mode),
        config,
        PhysAddr::None,
        application,
        information,
        control_handler,
    );
    crate::tcp::spawn_outstation_client(
        task,
        connect_handler,
        PostConnectionHandler::Tls(tls_config),
        listener,
        |name| tracing::info_span!("dnp3-outstation-tls-client", "endpoint" = ?name),
    );
    handle
}

/// TLS configuration for a server
pub struct TlsServerConfig {
    config: Arc<rustls::ServerConfig>,
}

impl std::fmt::Debug for TlsServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // the rustls configuration holds key material, so nothing is shown
        f.debug_struct("TlsServerConfig").finish_non_exhaustive()
    }
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

    pub(crate) async fn handle_connection<S: Socket>(
        &self,
        socket: S,
    ) -> Result<PhysLayer, HandshakeError> {
        let connector = tokio_rustls::TlsAcceptor::from(self.config.clone());
        // the server accepts one connection at a time, so a handshake that stalls would stop it
        // accepting any others
        let stream = super::handshake(connector.accept(socket)).await?;
        Ok(S::into_tls_phys(tokio_rustls::TlsStream::from(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcp::accept::test_util::PIPE;
    use crate::tcp::tls::TLS_HANDSHAKE_TIMEOUT;
    use crate::util::test_support::tls::server_config;
    use crate::util::test_support::watchdog;

    #[tokio::test(start_paused = true)]
    async fn tls_handshake_that_never_starts_times_out() {
        watchdog(async {
            let config = server_config(MinTlsVersion::V12);

            // a peer that connects and never sends its ClientHello
            let (socket, _peer) = tokio::io::duplex(PIPE);

            let start = tokio::time::Instant::now();
            let result = config.handle_connection(socket).await;
            assert!(matches!(result, Err(HandshakeError::Timeout)));
            assert_eq!(start.elapsed(), TLS_HANDSHAKE_TIMEOUT);
        })
        .await;
    }
}
