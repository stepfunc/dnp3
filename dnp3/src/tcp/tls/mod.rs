use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use tokio_rustls::rustls;
use tokio_rustls::webpki;

mod master;

pub use master::*;

/// TLS configuration
pub struct TlsConfig {
    /// DNS name to verify
    pub name: String,
    /// Path to the DER encoded peer certificate
    pub peer_cert_path: PathBuf,
    /// Path to the DER encoded local certificate
    pub local_cert_path: PathBuf,
    /// Path to the DER encoded private key associated with the local certificate
    pub private_key_path: PathBuf,
    /// Allow TLS 1.2 protocol
    pub allow_tls_1_2: bool,
    /// Allow TLS 1.3 protocol
    pub allow_tls_1_3: bool,
}

impl TlsConfig {
    /// Create a TLS config with TLS 1.2 and 1.3 support.
    pub fn new(name: String, peer_cert_path: PathBuf, local_cert_path: PathBuf, private_key_path: PathBuf) -> Self {
        Self {
            name,
            peer_cert_path,
            local_cert_path,
            private_key_path,
            allow_tls_1_2: true,
            allow_tls_1_3: true,
        }
    }

    pub(crate) fn to_client_config(&self) -> rustls::ClientConfig {
        let peer_certs = load_certs(&self.peer_cert_path);
        let local_certs = load_certs(&self.local_cert_path);
        let private_key = load_private_key(&self.private_key_path);

        let mut config = rustls::ClientConfig::new();

        // Add peer certificates
        for cert in &peer_certs {
            config.root_store.add(cert).unwrap();
        }

        // Set local cert chain
        config.set_single_client_cert(local_certs, private_key).unwrap();

        config.dangerous().set_certificate_verifier(Arc::new(CustomServerCertVerifier));

        // Set allowed TLS versions
        config.versions.clear();
        if self.allow_tls_1_2 {
            config.versions.push(rustls::ProtocolVersion::TLSv1_2);
        }
        if self.allow_tls_1_3 {
            config.versions.push(rustls::ProtocolVersion::TLSv1_3);
        }

        config
    }

    pub(crate) fn dns_name(&self) -> webpki::DNSNameRef {
        webpki::DNSNameRef::try_from_ascii_str(&self.name).unwrap()
    }
}

fn load_certs(path: &Path) -> Vec<rustls::Certificate> {
    let f = std::fs::File::open(path).unwrap();
    let mut f = std::io::BufReader::new(f);

    rustls_pemfile::certs(&mut f).unwrap().iter().map(|data| {
        rustls::Certificate(data.clone())
    }).collect()
}

fn load_private_key(path: &Path) -> rustls::PrivateKey {
    let f = std::fs::File::open(path).unwrap();
    let mut f = std::io::BufReader::new(f);

    match rustls_pemfile::read_one(&mut f).unwrap() {
        Some(rustls_pemfile::Item::RSAKey(key)) => rustls::PrivateKey(key),
        Some(rustls_pemfile::Item::PKCS8Key(key)) => rustls::PrivateKey(key),
        Some(rustls_pemfile::Item::X509Certificate(_)) => panic!("File contains cert, not private key"),
        None => panic!("No valid private key")
    }
}

struct CustomServerCertVerifier;

impl rustls::ServerCertVerifier for CustomServerCertVerifier {
    /// Copy-pasted from WebPKIVerifier impl
    /// 
    /// Will verify the certificate is valid in the following ways:
    /// - Signed by a  trusted `RootCertStore` CA
    /// - Not Expired
    /// - Valid for DNS entry
    /// - OCSP data is present
    fn verify_server_cert(
        &self,
        roots: &rustls::RootCertStore,
        presented_certs: &[rustls::Certificate],
        dns_name: webpki::DNSNameRef,
        ocsp_response: &[u8],
    ) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
        let (cert, chain, trustroots) = prepare(roots, presented_certs)?;
        let now = webpki::Time::try_from(std::time::SystemTime::now()).map_err(|_| rustls::TLSError::FailedToGetCurrentTime)?;
        let cert = match cert.verify_is_valid_tls_server_cert(
                SUPPORTED_SIG_ALGS,
                &webpki::TLSServerTrustAnchors(&trustroots),
                &chain,
                now,
            ) {
                Ok(_) => Ok(cert),
                // WARNING: THIS SKIPS SOME VALIDATIONS AND SHOULDN'T BE USED IN PRODUCTION
                // We ignore this error because we want to support self-signed certificate
                Err(webpki::Error::CAUsedAsEndEntity) => Ok(cert),
                Err(err) => Err(rustls::TLSError::WebPKIError(err))
            }?;

        if !ocsp_response.is_empty() {
            //trace!("Unvalidated OCSP response: {:?}", ocsp_response.to_vec());
        }

        cert.verify_is_valid_for_dns_name(dns_name)
            .map_err(rustls::TLSError::WebPKIError)
            .map(|_| rustls::ServerCertVerified::assertion())
    }
}

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];

static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::ED25519,
    &webpki::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

type CertChainAndRoots<'a, 'b> = (
    webpki::EndEntityCert<'a>,
    Vec<&'a [u8]>,
    Vec<webpki::TrustAnchor<'b>>,
);

fn prepare<'a, 'b>(
    roots: &'b rustls::RootCertStore,
    presented_certs: &'a [rustls::Certificate],
) -> Result<CertChainAndRoots<'a, 'b>, rustls::TLSError> {
    if presented_certs.is_empty() {
        return Err(rustls::TLSError::NoCertificatesPresented);
    }

    // EE cert must appear first.
    let cert = webpki::EndEntityCert::from(&presented_certs[0].0).map_err(rustls::TLSError::WebPKIError)?;

    let chain: Vec<&'a [u8]> = presented_certs
        .iter()
        .skip(1)
        .map(|cert| cert.0.as_ref())
        .collect();

    let trustroots: Vec<webpki::TrustAnchor> = roots
        .roots
        .iter()
        .map(rustls::OwnedTrustAnchor::to_trust_anchor)
        .collect();

    Ok((cert, chain, trustroots))
}
