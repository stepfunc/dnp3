use crate::tcp::tls::{pki_error, TlsError};
use tokio_rustls::{rustls, webpki};

pub(crate) struct NameVerifier {
    inner: VerifierType,
}

impl NameVerifier {
    pub(crate) fn try_create(name: String) -> Result<Self, TlsError> {
        if dangerous::allow_peer_name_wildcards() && name == "*" {
            Ok(NameVerifier {
                inner: VerifierType::Any,
            })
        } else {
            rustls::ServerName::try_from(name.as_str()).map_err(|_| TlsError::InvalidDnsName)?;
            Ok(NameVerifier {
                inner: VerifierType::Strict(name),
            })
        }
    }

    pub(crate) fn verify(&self, end_entity: &rustls::Certificate) -> Result<(), rustls::Error> {
        match &self.inner {
            VerifierType::Any => Ok(()),
            VerifierType::Strict(x) => verify_dns_name(end_entity, x.as_str()),
        }
    }
}

enum VerifierType {
    Any,
    Strict(String),
}

/// Configuration options related to dangerous (less-secure) modes that can only be used
/// after explicitly opting into them.
pub mod dangerous {

    use std::sync::atomic::Ordering;

    static ALLOW_WILDCARD_PEER_NAMES: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);

    /// Setting this option will allow the user to specify an "*" for the peer's name
    /// when creating a [crate::tcp::tls::TlsServerConfig]. This means that the any certificate
    /// signed by the CA will be allowed
    pub fn enable_peer_name_wildcards() {
        ALLOW_WILDCARD_PEER_NAMES.store(true, Ordering::Relaxed);
    }

    pub(crate) fn allow_peer_name_wildcards() -> bool {
        ALLOW_WILDCARD_PEER_NAMES.load(Ordering::Relaxed)
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
            let parsed_cert = rx509::x509::Certificate::parse(&cert.0).map_err(|err| {
                rustls::Error::InvalidCertificateData(format!(
                    "unable to parse cert with rasn: {err:?}"
                ))
            })?;

            // Parse the extensions (if present) and check that no SAN are present
            if let Some(extensions) = &parsed_cert.tbs_certificate.value.extensions {
                // Parse the extensions
                let extensions = extensions.parse().map_err(|err| {
                    rustls::Error::InvalidCertificateData(format!(
                        "unable to parse certificate extensions with rasn: {err:?}"
                    ))
                })?;

                // Check that no SAN extension are present
                if extensions.iter().any(|x| {
                    matches!(
                        x.content,
                        rx509::x509::ext::SpecificExtension::SubjectAlternativeName(_)
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
                        "unable to parse certificate subject: {err:?}"
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
