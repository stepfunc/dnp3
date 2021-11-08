mod master;
mod outstation;

use std::convert::TryFrom;
use std::io::{self, ErrorKind, Read};
use std::path::Path;

pub use master::*;
pub use outstation::*;
use tokio_rustls::{rustls, webpki};

/// Certificate validation mode
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum CertificateMode {
    /// Single or chain of certificates validated against trust anchors
    TrustChain,
    /// Single pre-shared self-sign certificate compared byte-for-byte
    SelfSignedCertificate,
}

/// TLS-related errors
#[derive(Debug)]
pub enum TlsError {
    /// Invalid peer certificate
    InvalidPeerCertificate(io::Error),
    /// Invalid local certificate
    InvalidLocalCertificate(io::Error),
    /// Invalid private key
    InvalidPrivateKey(io::Error),
    /// DNS name is invalid
    InvalidDnsName,
    /// Miscellaneous error
    Miscellaneous(io::Error),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPeerCertificate(err) => {
                write!(f, "invalid peer certificate file: {}", err)
            }
            Self::InvalidLocalCertificate(err) => {
                write!(f, "invalid local certificate file: {}", err)
            }
            Self::InvalidPrivateKey(err) => write!(f, "invalid private key file: {}", err),
            Self::InvalidDnsName => write!(f, "invalid DNS name"),
            Self::Miscellaneous(err) => write!(f, "miscellaneous TLS error: {}", err),
        }
    }
}

impl std::error::Error for TlsError {}

/// Minimum TLS version to allow
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub enum MinTlsVersion {
    /// TLS 1.2
    Tls1_2,
    /// TLS 1.3
    Tls1_3,
}

impl MinTlsVersion {
    fn to_rustls(self) -> &'static [&'static rustls::SupportedProtocolVersion] {
        static MIN_TLS12_VERSIONS: &[&rustls::SupportedProtocolVersion] =
            &[&rustls::version::TLS13, &rustls::version::TLS12];
        static MIN_TLS13_VERSIONS: &[&rustls::SupportedProtocolVersion] =
            &[&rustls::version::TLS13];

        match self {
            Self::Tls1_2 => MIN_TLS12_VERSIONS,
            Self::Tls1_3 => MIN_TLS13_VERSIONS,
        }
    }
}

fn verify_dns_name(cert: &rustls::Certificate, server_name: &str) -> Result<(), rustls::Error> {
    // Extract the DNS name
    /*let dns_name_str = match server_name {
        rustls::ServerName::DnsName(dns_name) => dns_name.as_ref(),
        &_ => {
            // I don't undertand why I need this, but the compiler keeps complaining otherwise
            unreachable!()
        }
    };*/
    let dns_name = webpki::DnsNameRef::try_from_ascii_str(server_name)
        .map_err(|_| rustls::Error::InvalidCertificateData("invalid DNS name".to_string()))?;

    // Let webpki parse the cert
    let end_entity_cert = webpki::EndEntityCert::try_from(cert.0.as_ref()).map_err(pki_error)?;

    // Try validating the name using webpki. This only checks SAN extensions
    match end_entity_cert.verify_is_valid_for_dns_name(dns_name) {
        Ok(()) => Ok(()), // Good, we found a SAN extension that fits for the DNS name
        Err(webpki::Error::CertNotValidForName) => {
            // Let's extend our search to the CN
            // Parse the certificate using rasn
            let parsed_cert = rasn::x509::Certificate::parse(&cert.0).map_err(|err| {
                rustls::Error::InvalidCertificateData(format!(
                    "unable to parse cert with rasn: {:?}",
                    err
                ))
            })?;

            // Parse the extensions (if present) and check that no SAN are present
            if let Some(extensions) = &parsed_cert.tbs_certificate.value.extensions {
                // Parse the extensions
                let extensions = extensions.parse().map_err(|err| {
                    rustls::Error::InvalidCertificateData(format!(
                        "unable to parse certificate extensions with rasn: {:?}",
                        err
                    ))
                })?;

                // Check that no SAN extension are present
                if extensions.iter().any(|x| {
                    matches!(
                        x.content,
                        rasn::extensions::SpecificExtension::SubjectAlternativeName(_)
                    )
                }) {
                    return Err(rustls::Error::InvalidCertificateData(
                        "certificate not valid for name, SAN extensions do not match".to_string(),
                    ));
                }
            }

            // Parse the cert subject
            let subject = parsed_cert
                .tbs_certificate
                .value
                .subject
                .parse()
                .map_err(|err| {
                    rustls::Error::InvalidCertificateData(format!(
                        "unable to parse certificate subject: {:?}",
                        err
                    ))
                })?;

            let common_name = subject.common_name.ok_or_else(|| {
                rustls::Error::InvalidCertificateData(
                    "certificate not valid for name, no SAN and no CN present".to_string(),
                )
            })?;

            match common_name == server_name {
                true => Ok(()),
                false => Err(rustls::Error::InvalidCertificateData(
                    "certificate not valid for name, no SAN and CN doesn't match".to_string(),
                )),
            }
        }
        Err(err) => Err(pki_error(err)), // Any other error means there was an error parsing the cert, we should throw
    }
}

fn pki_error(error: webpki::Error) -> rustls::Error {
    use webpki::Error::*;
    match error {
        BadDer | BadDerTime => rustls::Error::InvalidCertificateEncoding,
        InvalidSignatureForPublicKey => rustls::Error::InvalidCertificateSignature,
        UnsupportedSignatureAlgorithm | UnsupportedSignatureAlgorithmForPublicKey => {
            rustls::Error::InvalidCertificateSignatureType
        }
        e => rustls::Error::InvalidCertificateData(format!("invalid peer certificate: {}", e)),
    }
}

fn load_certs(path: &Path, is_local: bool) -> Result<Vec<rustls::Certificate>, TlsError> {
    let map_error_fn = match is_local {
        false => TlsError::InvalidPeerCertificate,
        true => TlsError::InvalidLocalCertificate,
    };

    let f = std::fs::File::open(path).map_err(map_error_fn)?;
    let mut f = io::BufReader::new(f);

    let certs = rustls_pemfile::certs(&mut f)
        .map_err(map_error_fn)?
        .iter()
        .map(|data| rustls::Certificate(data.clone()))
        .collect::<Vec<_>>();

    match certs.len() {
        0 => Err(map_error_fn(io::Error::new(
            ErrorKind::InvalidData,
            "no certificate in pem file",
        ))),
        _ => Ok(certs),
    }
}

fn load_private_key(path: &Path, password: Option<&str>) -> Result<rustls::PrivateKey, TlsError> {
    let f = std::fs::File::open(path).map_err(TlsError::InvalidPrivateKey)?;
    let mut f = io::BufReader::new(f);

    match password {
        // With a password, we parse using pkcs8
        Some(password) => {
            let mut file_content = String::new();
            f.read_to_string(&mut file_content)
                .map_err(TlsError::InvalidPrivateKey)?;
            let encrypted = pkcs8::EncryptedPrivateKeyDocument::from_pem(&file_content)?;
            let decrypted = encrypted.decrypt(password)?;
            Ok(rustls::PrivateKey(decrypted.as_ref().to_owned()))
        }
        // No password, we parse using rustls-pemfile
        None => match rustls_pemfile::read_one(&mut f).map_err(TlsError::InvalidPrivateKey)? {
            Some(rustls_pemfile::Item::RSAKey(key)) => Ok(rustls::PrivateKey(key)),
            Some(rustls_pemfile::Item::PKCS8Key(key)) => Ok(rustls::PrivateKey(key)),
            Some(rustls_pemfile::Item::X509Certificate(_)) => {
                Err(TlsError::InvalidPrivateKey(io::Error::new(
                    ErrorKind::InvalidData,
                    "file contains cert, not private key",
                )))
            }
            None => Err(TlsError::InvalidPrivateKey(io::Error::new(
                ErrorKind::InvalidData,
                "file does not contain private key",
            ))),
        },
    }
}

impl From<pkcs8::Error> for TlsError {
    fn from(from: pkcs8::Error) -> Self {
        TlsError::InvalidPrivateKey(io::Error::new(ErrorKind::InvalidData, from.to_string()))
    }
}
