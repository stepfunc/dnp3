use std::io::{self, ErrorKind};
use std::path::Path;

use tokio_rustls::rustls;

use crate::tcp::tls::{load_certs, load_private_key, TlsError};
use crate::tokio::net::TcpStream;
use crate::util::phys::PhysLayer;

/// TLS configuration
pub struct TlsServerConfig {
    config: std::sync::Arc<rustls::ServerConfig>,
}

impl TlsServerConfig {
    /// Create a TLS server config
    pub fn new(
        peer_cert_path: &Path,
        local_cert_path: &Path,
        private_key_path: &Path,
        allow_tls_1_2: bool,
    ) -> Result<Self, TlsError> {
        let peer_certs = load_certs(peer_cert_path, false)?;
        let local_certs = load_certs(local_cert_path, true)?;
        let private_key = load_private_key(private_key_path)?;

        // Build root certificate store
        let mut root_cert_store = rustls::RootCertStore::empty();
        for cert in &peer_certs {
            root_cert_store.add(cert).map_err(|err| {
                TlsError::InvalidPeerCertificate(io::Error::new(
                    ErrorKind::InvalidData,
                    err.to_string(),
                ))
            })?;
        }

        // TODO: do we want to do name checking here? I'm not sure how to do it...
        let client_cert_verifier = rustls::AllowAnyAuthenticatedClient::new(root_cert_store);
        let mut config = rustls::ServerConfig::new(client_cert_verifier);

        // Set local cert chain
        config
            .set_single_cert(local_certs, private_key)
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

        Ok(Self {
            config: std::sync::Arc::new(config),
        })
    }

    pub(crate) async fn handle_connection(
        &mut self,
        socket: TcpStream,
    ) -> Result<PhysLayer, String> {
        let connector = tokio_rustls::TlsAcceptor::from(self.config.clone());
        match connector.accept(socket).await {
            Err(err) => Err(format!("failed to establish TLS session: {}", err)),
            Ok(stream) => Ok(PhysLayer::Tls(Box::new(tokio_rustls::TlsStream::from(
                stream,
            )))),
        }
    }
}
