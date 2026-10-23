// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Handshakes over an in-memory pipe that pin how the client and server configurations apply
//! each certificate mode, the subject name and the minimum TLS version

use std::future::Future;
use std::sync::Arc;

use sfio_rustls_config::{ClientNameVerification, ProtocolVersions, ServerNameVerification};
use tokio::io::DuplexStream;
use tokio_rustls::rustls;
use tokio_rustls::rustls::client::ResolvesClientCert;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::sign::CertifiedKey;
use tokio_rustls::rustls::{
    AlertDescription, CertificateError, PeerIncompatible, ProtocolVersion, SignatureScheme,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::tcp::accept::test_util::PIPE;
use crate::tcp::tls::{
    CertificateMode, HandshakeError, MinTlsVersion, TlsClientConfig, TlsServerConfig,
};
use crate::util::test_support::tls::{
    ca_chain, cert, client_config, rustls_client, rustls_server, server_config,
};
use crate::util::test_support::watchdog;

/// The subject name of both certificates the authority issued
const NAME: &str = "test.com";

const MODES: [CertificateMode; 2] = [CertificateMode::AuthorityBased, CertificateMode::SelfSigned];

/// A client presenting the first entity's certificate
fn client(mode: CertificateMode, min: MinTlsVersion) -> TlsClientConfig {
    match mode {
        CertificateMode::AuthorityBased => pki_client(Some(NAME), "ca_cert.pem", min),
        CertificateMode::SelfSigned => client_config(min),
    }
}

/// A server presenting the second entity's certificate
fn server(mode: CertificateMode, min: MinTlsVersion) -> TlsServerConfig {
    match mode {
        CertificateMode::AuthorityBased => pki_server(Some(NAME), &ca_chain("ca_cert.pem"), min),
        CertificateMode::SelfSigned => server_config(min),
    }
}

/// A full-PKI client presenting the first entity's certificate and trusting `authority`
fn pki_client(name: Option<&str>, authority: &str, min: MinTlsVersion) -> TlsClientConfig {
    TlsClientConfig::full_pki(
        name.map(str::to_string),
        &ca_chain(authority),
        &ca_chain("entity1_cert.pem"),
        &ca_chain("entity1_key.pem"),
        None,
        min,
    )
    .unwrap()
}

/// A full-PKI server presenting the second entity's certificate and trusting `authority`
fn pki_server(
    name: Option<&str>,
    authority: &std::path::Path,
    min: MinTlsVersion,
) -> TlsServerConfig {
    TlsServerConfig::full_pki(
        name.map(str::to_string),
        authority,
        &ca_chain("entity2_cert.pem"),
        &ca_chain("entity2_key.pem"),
        None,
        min,
    )
    .unwrap()
}

/// A peer that would complete a handshake with [`server`], built without this crate's configuration
fn peer_client(mode: CertificateMode, versions: ProtocolVersions) -> rustls::ClientConfig {
    match mode {
        CertificateMode::AuthorityBased => sfio_rustls_config::client::authority(
            versions,
            ServerNameVerification::SanOrCommonName,
            &ca_chain("ca_cert.pem"),
            &ca_chain("entity1_cert.pem"),
            &ca_chain("entity1_key.pem"),
            None,
        )
        .unwrap(),
        CertificateMode::SelfSigned => rustls_client(versions),
    }
}

/// A peer that would complete a handshake with [`client`], built without this crate's configuration
fn peer_server(mode: CertificateMode, versions: ProtocolVersions) -> rustls::ServerConfig {
    match mode {
        CertificateMode::AuthorityBased => sfio_rustls_config::server::authority(
            versions,
            ClientNameVerification::None,
            &ca_chain("ca_cert.pem"),
            &ca_chain("entity2_cert.pem"),
            &ca_chain("entity2_key.pem"),
            None,
        )
        .unwrap(),
        CertificateMode::SelfSigned => rustls_server(versions),
    }
}

fn tls12_only() -> ProtocolVersions {
    ProtocolVersions::new().enable_v12()
}

fn any_version() -> ProtocolVersions {
    MinTlsVersion::V12.into()
}

/// Replaces the certificate chain a peer client presents
#[derive(Debug)]
enum ClientChain {
    /// Present no certificate
    Nothing,
    /// Present the configured certificate with a copy of itself as an intermediate
    WithIntermediate(Arc<dyn ResolvesClientCert>),
}

impl ResolvesClientCert for ClientChain {
    fn resolve(
        &self,
        root_hint_subjects: &[&[u8]],
        sigschemes: &[SignatureScheme],
    ) -> Option<Arc<CertifiedKey>> {
        match self {
            Self::Nothing => None,
            Self::WithIntermediate(inner) => {
                let key = inner.resolve(root_hint_subjects, sigschemes)?;
                let chain = key.cert.iter().chain(key.cert.iter()).cloned().collect();
                Some(Arc::new(CertifiedKey::new(chain, key.key.clone())))
            }
        }
    }

    fn has_certs(&self) -> bool {
        matches!(self, Self::WithIntermediate(_))
    }
}

/// Run both sides of a handshake over a pipe
async fn handshake<C: Future, S: Future>(
    client: impl FnOnce(DuplexStream) -> C,
    server: impl FnOnce(DuplexStream) -> S,
) -> (C::Output, S::Output) {
    let (client_io, server_io) = tokio::io::duplex(PIPE);
    tokio::join!(client(client_io), server(server_io))
}

/// Connect a peer client to `io`
async fn connect_peer(
    config: rustls::ClientConfig,
    io: DuplexStream,
) -> std::io::Result<tokio_rustls::client::TlsStream<DuplexStream>> {
    let name = ServerName::try_from(NAME).unwrap();
    TlsConnector::from(Arc::new(config)).connect(name, io).await
}

/// Accept a peer server's side of a handshake on `io`
async fn accept_peer(
    config: rustls::ServerConfig,
    io: DuplexStream,
) -> std::io::Result<tokio_rustls::server::TlsStream<DuplexStream>> {
    TlsAcceptor::from(Arc::new(config)).accept(io).await
}

/// The TLS error that ended a handshake
fn tls_error<T>(result: Result<T, HandshakeError>) -> rustls::Error {
    match result {
        Ok(_) => panic!("the handshake succeeded"),
        Err(HandshakeError::Timeout) => panic!("the handshake timed out"),
        Err(HandshakeError::Failed(err)) => *err
            .into_inner()
            .expect("an error without a cause")
            .downcast::<rustls::Error>()
            .expect("not a TLS error"),
    }
}

#[tokio::test]
async fn authority_based_peers_with_matching_names_complete_the_handshake() {
    watchdog(async {
        let client = client(CertificateMode::AuthorityBased, MinTlsVersion::V12);
        let server = server(CertificateMode::AuthorityBased, MinTlsVersion::V12);

        let (client, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        let client = client.unwrap();
        server.unwrap();
        assert_eq!(
            client.get_ref().1.protocol_version(),
            Some(ProtocolVersion::TLSv1_3)
        );
    })
    .await;
}

#[tokio::test]
async fn self_signed_peers_with_each_others_certificates_complete_the_handshake() {
    watchdog(async {
        let client = client(CertificateMode::SelfSigned, MinTlsVersion::V12);
        let server = server(CertificateMode::SelfSigned, MinTlsVersion::V12);

        let (client, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        client.unwrap();
        server.unwrap();
    })
    .await;
}

#[tokio::test]
async fn without_subject_names_authority_based_peers_accept_any_trusted_name() {
    watchdog(async {
        let client = pki_client(None, "ca_cert.pem", MinTlsVersion::V12);
        let server = pki_server(None, &ca_chain("ca_cert.pem"), MinTlsVersion::V12);

        let (client, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        client.unwrap();
        server.unwrap();
    })
    .await;
}

#[tokio::test]
async fn client_rejects_a_server_with_another_name() {
    watchdog(async {
        let client = pki_client(Some("other.com"), "ca_cert.pem", MinTlsVersion::V12);
        let server = server(CertificateMode::AuthorityBased, MinTlsVersion::V12);

        let (client, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        assert_eq!(
            tls_error(client),
            rustls::Error::InvalidCertificate(CertificateError::NotValidForName)
        );
        server.unwrap_err();
    })
    .await;
}

#[tokio::test]
async fn server_rejects_a_client_with_another_name() {
    watchdog(async {
        let client = client(CertificateMode::AuthorityBased, MinTlsVersion::V12);
        let server = pki_server(
            Some("other.com"),
            &ca_chain("ca_cert.pem"),
            MinTlsVersion::V12,
        );

        let (_, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        assert_eq!(
            tls_error(server),
            rustls::Error::InvalidCertificate(CertificateError::NotValidForName)
        );
    })
    .await;
}

#[tokio::test]
async fn client_without_a_subject_name_rejects_a_server_from_another_authority() {
    watchdog(async {
        // a self-signed CA certificate that didn't issue the server's certificate
        let client = TlsClientConfig::full_pki(
            None,
            &cert("entity1_cert.pem"),
            &ca_chain("entity1_cert.pem"),
            &ca_chain("entity1_key.pem"),
            None,
            MinTlsVersion::V12,
        )
        .unwrap();
        let server = server(CertificateMode::AuthorityBased, MinTlsVersion::V12);

        let (client, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        assert_eq!(
            tls_error(client),
            rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer)
        );
        server.unwrap_err();
    })
    .await;
}

#[tokio::test]
async fn server_without_a_subject_name_rejects_a_client_from_another_authority() {
    watchdog(async {
        let client = client(CertificateMode::AuthorityBased, MinTlsVersion::V12);
        // a self-signed CA certificate that didn't issue the client's certificate
        let server = pki_server(None, &cert("entity1_cert.pem"), MinTlsVersion::V12);

        let (_, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        assert_eq!(
            tls_error(server),
            rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer)
        );
    })
    .await;
}

#[tokio::test]
async fn self_signed_client_rejects_a_server_certificate_it_was_not_given() {
    watchdog(async {
        // expects the client's own certificate instead of the server's
        let client = TlsClientConfig::self_signed(
            &cert("entity1_cert.pem"),
            &cert("entity1_cert.pem"),
            &cert("entity1_key.pem"),
            None,
            MinTlsVersion::V12,
        )
        .unwrap();
        let server = server(CertificateMode::SelfSigned, MinTlsVersion::V12);

        let (client, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        assert_eq!(
            tls_error(client),
            rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer)
        );
        server.unwrap_err();
    })
    .await;
}

#[tokio::test]
async fn self_signed_server_rejects_a_client_certificate_it_was_not_given() {
    watchdog(async {
        let client = client(CertificateMode::SelfSigned, MinTlsVersion::V12);
        // expects the server's own certificate instead of the client's
        let server = TlsServerConfig::self_signed(
            &cert("entity2_cert.pem"),
            &cert("entity2_cert.pem"),
            &cert("entity2_key.pem"),
            None,
            MinTlsVersion::V12,
        )
        .unwrap();

        let (_, server) =
            handshake(|io| client.connect(io), |io| server.handle_connection(io)).await;

        assert_eq!(
            tls_error(server),
            rustls::Error::InvalidCertificate(CertificateError::UnknownIssuer)
        );
    })
    .await;
}

#[tokio::test]
async fn self_signed_server_rejects_a_client_that_sends_intermediates() {
    watchdog(async {
        let server = server(CertificateMode::SelfSigned, MinTlsVersion::V12);
        let mut peer = peer_client(CertificateMode::SelfSigned, any_version());
        peer.client_auth_cert_resolver = Arc::new(ClientChain::WithIntermediate(
            peer.client_auth_cert_resolver.clone(),
        ));

        let (_, server) = handshake(
            |io| connect_peer(peer, io),
            |io| server.handle_connection(io),
        )
        .await;

        assert!(matches!(tls_error(server), rustls::Error::General(_)));
    })
    .await;
}

#[tokio::test]
async fn servers_reject_a_client_without_a_certificate() {
    watchdog(async {
        for mode in MODES {
            let server = server(mode, MinTlsVersion::V12);
            let mut peer = peer_client(mode, any_version());
            peer.client_auth_cert_resolver = Arc::new(ClientChain::Nothing);

            let (_, server) = handshake(
                |io| connect_peer(peer, io),
                |io| server.handle_connection(io),
            )
            .await;

            assert_eq!(tls_error(server), rustls::Error::NoCertificatesPresented);
        }
    })
    .await;
}

#[tokio::test]
async fn servers_with_minimum_version_1_3_reject_a_1_2_only_client() {
    watchdog(async {
        for mode in MODES {
            let server = server(mode, MinTlsVersion::V13);
            let peer = peer_client(mode, tls12_only());

            let (peer, server) = handshake(
                |io| connect_peer(peer, io),
                |io| server.handle_connection(io),
            )
            .await;

            assert_eq!(
                tls_error(server),
                rustls::Error::PeerIncompatible(PeerIncompatible::Tls12NotOfferedOrEnabled)
            );
            peer.unwrap_err();
        }
    })
    .await;
}

#[tokio::test]
async fn clients_with_minimum_version_1_3_reject_a_1_2_only_server() {
    watchdog(async {
        for mode in MODES {
            let client = client(mode, MinTlsVersion::V13);
            let peer = peer_server(mode, tls12_only());

            let (client, peer) =
                handshake(|io| client.connect(io), |io| accept_peer(peer, io)).await;

            assert_eq!(
                tls_error(client),
                rustls::Error::AlertReceived(AlertDescription::ProtocolVersion)
            );
            peer.unwrap_err();
        }
    })
    .await;
}

#[tokio::test]
async fn minimum_version_1_2_negotiates_1_2_with_a_1_2_only_peer() {
    watchdog(async {
        for mode in MODES {
            let client = client(mode, MinTlsVersion::V12);
            let peer = peer_server(mode, tls12_only());

            let (client, peer) =
                handshake(|io| client.connect(io), |io| accept_peer(peer, io)).await;

            assert_eq!(
                client.unwrap().get_ref().1.protocol_version(),
                Some(ProtocolVersion::TLSv1_2)
            );
            peer.unwrap();

            let server = server(mode, MinTlsVersion::V12);
            let peer = peer_client(mode, tls12_only());

            let (peer, server) = handshake(
                |io| connect_peer(peer, io),
                |io| server.handle_connection(io),
            )
            .await;

            server.unwrap();
            assert_eq!(
                peer.unwrap().get_ref().1.protocol_version(),
                Some(ProtocolVersion::TLSv1_2)
            );
        }
    })
    .await;
}
