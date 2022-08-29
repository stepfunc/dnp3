mod master;
mod outstation;

use std::convert::TryFrom;
use std::io::{self, ErrorKind};
use std::path::Path;

pub use master::*;
pub use outstation::*;
use tokio_rustls::{rustls, webpki};

/// Determines how the certificate(s) presented by the peer are validated
///
/// This validation always occurs **after** the handshake signature has been
/// verified.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertificateMode {
    /// Validates the peer certificate against one or more configured trust anchors
    ///
    /// This mode uses the default certificate verifier in `rustls` to ensure that
    /// the chain of certificates presented by the peer is valid against one of
    /// the configured trust anchors.
    ///
    /// The name verification is relaxed to allow for certificates that do not contain
    /// the SAN extension. In these cases the name is verified using the Common Name instead.
    AuthorityBased,
    /// Validates that the peer presents a single certificate which is a byte-for-byte match
    /// against the configured peer certificate.
    ///
    /// The certificate is parsed only to ensure that the `NotBefore` and `NotAfter`
    /// are valid for the current system time.
    SelfSigned,
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
    /// Other error
    Other(io::Error),
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
            Self::Other(err) => write!(f, "miscellaneous TLS error: {}", err),
        }
    }
}

impl std::error::Error for TlsError {}

/// Minimum TLS version to allow
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MinTlsVersion {
    /// TLS 1.2
    V12,
    /// TLS 1.3
    V13,
}

impl MinTlsVersion {
    fn to_rustls(self) -> &'static [&'static rustls::SupportedProtocolVersion] {
        static MIN_TLS12_VERSIONS: &[&rustls::SupportedProtocolVersion] =
            &[&rustls::version::TLS13, &rustls::version::TLS12];
        static MIN_TLS13_VERSIONS: &[&rustls::SupportedProtocolVersion] =
            &[&rustls::version::TLS13];

        match self {
            Self::V12 => MIN_TLS12_VERSIONS,
            Self::V13 => MIN_TLS13_VERSIONS,
        }
    }
}

fn verify_dns_name(cert: &rustls::Certificate, server_name: &str) -> Result<(), rustls::Error> {
    // Extract the DNS name
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

    let content = std::fs::read(path).map_err(map_error_fn)?;
    let certs = pem::parse_many(content)
        .map_err(|err| map_error_fn(io::Error::new(ErrorKind::InvalidData, err.to_string())))?
        .into_iter()
        .filter(|x| x.tag == "CERTIFICATE")
        .map(|x| rustls::Certificate(x.contents))
        .collect::<Vec<_>>();

    if certs.is_empty() {
        return Err(map_error_fn(io::Error::new(
            ErrorKind::InvalidData,
            "no certificate in pem file",
        )));
    }

    Ok(certs)
}

fn load_private_key(path: &Path, password: Option<&str>) -> Result<rustls::PrivateKey, TlsError> {
    let expected_tag = match &password {
        Some(_) => "ENCRYPTED PRIVATE KEY",
        None => "PRIVATE KEY",
    };

    let content = std::fs::read(path).map_err(TlsError::InvalidPrivateKey)?;
    let mut iter = pem::parse_many(content)
        .map_err(|err| {
            TlsError::InvalidPrivateKey(io::Error::new(ErrorKind::InvalidData, err.to_string()))
        })?
        .into_iter()
        .filter(|x| x.tag == expected_tag)
        .map(|x| x.contents);

    let key = match iter.next() {
        Some(key) => match password {
            Some(password) => {
                let encrypted = pkcs8::EncryptedPrivateKeyDocument::from_der(&key)?;
                let decrypted = encrypted.decrypt(password)?;
                rustls::PrivateKey(decrypted.as_ref().to_owned())
            }
            None => rustls::PrivateKey(key),
        },
        None => {
            return Err(TlsError::InvalidPrivateKey(io::Error::new(
                ErrorKind::InvalidData,
                "no private key found in PEM file",
            )));
        }
    };

    // Check that there are no other keys
    if iter.next().is_some() {
        return Err(TlsError::InvalidPrivateKey(io::Error::new(
            ErrorKind::InvalidData,
            "more than one private key is present in the PEM file",
        )));
    }

    Ok(key)
}

impl From<pkcs8::Error> for TlsError {
    fn from(from: pkcs8::Error) -> Self {
        TlsError::InvalidPrivateKey(io::Error::new(ErrorKind::InvalidData, from.to_string()))
    }
}
