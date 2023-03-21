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
                write!(f, "invalid peer certificate file: {err}")
            }
            Self::InvalidLocalCertificate(err) => {
                write!(f, "invalid local certificate file: {err}")
            }
            Self::InvalidPrivateKey(err) => write!(f, "invalid private key file: {err}"),
            Self::InvalidDnsName => write!(f, "invalid DNS name"),
            Self::Other(err) => write!(f, "miscellaneous TLS error: {err}"),
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

fn pki_error(error: webpki::Error) -> rustls::Error {
    use webpki::Error::*;
    match error {
        BadDer | BadDerTime => rustls::Error::InvalidCertificateEncoding,
        InvalidSignatureForPublicKey => rustls::Error::InvalidCertificateSignature,
        UnsupportedSignatureAlgorithm | UnsupportedSignatureAlgorithmForPublicKey => {
            rustls::Error::InvalidCertificateSignatureType
        }
        e => rustls::Error::InvalidCertificateData(format!("invalid peer certificate: {e}")),
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
    let expected_tags: &[&'static str] = match &password {
        Some(_) => &["ENCRYPTED PRIVATE KEY"],
        None => &["PRIVATE KEY", "BEGIN RSA PRIVATE KEY"],
    };

    let content = std::fs::read(path).map_err(TlsError::InvalidPrivateKey)?;
    let mut iter = pem::parse_many(content)
        .map_err(|err| {
            TlsError::InvalidPrivateKey(io::Error::new(ErrorKind::InvalidData, err.to_string()))
        })?
        .into_iter()
        .filter(|x| expected_tags.iter().any(|t| x.tag.as_str() == *t))
        .map(|x| x.contents);

    let key = match iter.next() {
        Some(key) => match password {
            Some(password) => {
                let key = DecryptedKey::parse_and_decrypt(&key, password)?;
                rustls::PrivateKey(key.as_bytes().to_owned())
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

struct DecryptedKey {
    inner: pkcs8::SecretDocument,
}

impl DecryptedKey {
    fn parse_and_decrypt(der: &[u8], password: &str) -> Result<Self, TlsError> {
        let parsed = pkcs8::EncryptedPrivateKeyInfo::try_from(der)?;
        Ok(Self {
            inner: parsed.decrypt(password)?,
        })
    }

    fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl From<pkcs8::der::Error> for TlsError {
    fn from(from: pkcs8::der::Error) -> Self {
        TlsError::InvalidPrivateKey(io::Error::new(ErrorKind::InvalidData, from.to_string()))
    }
}

impl From<pkcs8::Error> for TlsError {
    fn from(from: pkcs8::Error) -> Self {
        TlsError::InvalidPrivateKey(io::Error::new(ErrorKind::InvalidData, from.to_string()))
    }
}

#[cfg(test)]
mod test {
    use crate::tcp::tls::DecryptedKey;

    const TEST_KEY: &str = r#"
-----BEGIN ENCRYPTED PRIVATE KEY-----
MIIFJTBPBgkqhkiG9w0BBQ0wQjAhBgkrBgEEAdpHBAswFAQIu0Ufk06Ty+ICAkAA
AgEIAgEBMB0GCWCGSAFlAwQBKgQQIP2MoQA/IuQ9YgoLJAHEnQSCBNBMF6XHpHn8
lKR0MfyeCPi1bGgpp39c6s3he9WdB57Z9r9SLrACbdMeLzOfbr5hF2JmCYk0T7Us
p6s20q5tiwd9zDWAHbGKOnzpVlLJhhz4GvfaTVt6K0onPt7Y3mfB9P44G3p3j83Z
3Ekg784DH26gYgIYK8uo0PNnBZbuoVTiBdj2BtsJpBysoztPeCEbF6xjw4p8obEo
YYOH2djLOfWipr0iqIdX6IAPQ/zKjAkZHy1VwEYcSmE5YS6UbzUkzFGEt4tV9RYJ
4ctJO2PVpFvmdmVvYCrzJI6BHX0a1AYd5FV/j+2Hd/wnRhp2srLv/rsYHF/J6VK0
E6FuLjLDHcG4TthQSlGJ/ewT6xiSZ7HdDn3BJfOJR9d9V8f+FfpEVL55lzPOh9Fl
Ad8cNClliziJCgkAJ7MhUenNwTnxOQYrnVDDUMTNYEiK6EoUWlSDbLp13n/uhmaw
huXvfLAMSmfJvWAEcnXn7X+/mHkEMWtrZkvHg2yDfyTk/8ZtRIw25+hp6MOBpbtn
7py2QlefjDMa5wAQGqACVNkMFUzEpNBNh1hj3CHBYdE1Jo5dyZPFcuGWY8ye75RI
F3Lq/z3NqbaebPUmXlpLh0YSvpyoZJM+Knr6bCGWj1Ik3oq0+gqbGmPQKULpmdkz
5JgD0TsD/yQL6Ldm1KMzhJmwovH25YIxcrbGmlQ2658XGNS/3kPR0UbdIgR4unWz
XcwV75HKT4Rc0E0fAYKzFVE9J08aQawhNKbaGGc8zNQiz9tGSVvpO3OLf35tHoPz
eRs4flWGR/b/seGeEcVzeO5RDNDxXblfoSc/gB7lPdA35ig9z6egQvUyJrmIUhGa
lRKg9UeXsT2gZZHZPCaYekWacBtNWYKzdrdgSHxxKkjvF/tWKxE62RaWzuwqs0qK
tEa7RBKwe+wYRp4KVWe15XO5dvfYYtGkza70QYhQw86foAtYHpI6nMv4ppBf/vkD
mcivoGnImRMGlt1Klo1I3VjKI3lvms9UpmNuI35THwxnz5O1aCvS1pBvXEs+D95y
4qvhVkcbYMF3anxgn0ZD+3LYTxWuPbRBxh6GxC4qbn9tPHtN/7Iop7pSqaQMBFt2
1pe/PuGpiswQY3mtU3WLVP4pC0Mu5KDShswRQmI7XLOIjQgT5ac3JrFogqgkX+rb
9ZS7jHDBNY7eGsI7sLiMVRnHltiOOwhBHu9+NAwi1jmJIvSPLJzf/MW6rRnpg10D
pHC/LdUlon31MBb5kqpidhQa1LD9gWzesMLq3DMkUbTAbY0sddSP8XkUidLsZEMx
mjmaNlfKhLNE0N7o59t40+l3W2bghnsd/VC9fZL66ShSISe3bzHjnJklpLNHhl3/
gFwoMvSG8u6Aboz5QfXFS5HliIf5Fnw8ed+deEb5z3fiSpKi0PbcVRzQWotzG152
FKueCGIlqyTzlh7j8wY8tnChbY/34kIGFUDtKQcAjR6mLSi80s53dBG/lIZAEsG5
P1x3xO3fzO5w9X5yrRxGnN6N6McMqTkF1V4DhUfwrb41QBR2UBdbed126ZhM52fc
9We9YmUyRcV38yXPJ6SB6I9xdNIa5Nv4tB/7DzvpaRS6FBUPVm+VsULxc2XIjHGp
LOCHvWcZO49jGhZErBtxk16H58koZVg5Zw==
-----END ENCRYPTED PRIVATE KEY-----"#;

    const DECYPTED_KEY: &str = r#"
-----BEGIN PRIVATE KEY-----
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQDOX/fVrsEE5ihY
BIi97q2u1j7lMx+3plyDBOzjK7uovVtmGadtmfLxtRvrFYG4tdUNZ6svhiFvutNW
GQxT4RWw//UkhnS3Znh+VhR0C4qsrMARP3svugFqLkwqS36jWLAy3bvVXlkxsKKi
Tjf5DQwrgM2s6s7QBYMJNN8+jBsYlZXf+5bUN4pFk6vSU7NFPZUSBZ/u3rSa4mR6
aD0QRMEYW4+fjj7+g+aZgUHHQT8Cz1FKCAbHSgVugOVC7Pwn3L9ozH2RC/cQdDhD
Arq+P1o7+4YyYG12bp/D5lFu+kdT6p0mHSx6l11sEAEIUGGeI/685K3GwZ4t2ip0
qQ+/xbVdAgMBAAECggEBAJjq8WgrWii4JjK6AVzDK3z+kZIhpKHfKnOGxcS6lg29
aako3y/OP/8r1KkHwZxNV7XcGDNZrxLsG0aTvte0U+9YaZwL6RYwXp42SGeIWdQD
GTpukGfX6s5zycoZMJf20nCObmz2wR6ZpJihXsYzDc56XWyAfIgVXXgH7leZV0aJ
08pEv0kAHGZSrxHGBT3n6+wrkHduTZ89Wx3kubwJRiRtHp8IdNhNoxlN44e5uizu
7ix6k/pg5TDaXOwsp4cE8SQ3zUa+kvazX693sCZdhCKlmrwYP9WZCAWLAy8sRFM+
SAj2cNWva1LL/hE3pgrCg4BV9zmr0UFMSLJxdcDAQZ0CgYEA69qUzzWU8KIKmXf5
J7yWKbUftXwIn4DrlabrpeWCYsvxfNsZWYEc+8VqZDczwwI2nf9useXQhfxH7VSc
q5d20ApNY3hHemp6s1p9224oj/6rKJtPBOdSIyAs1cOjNYK+YXcVjYlWHvHBpraL
ct2vo8yTkPkLAOJEqckPbadkVFsCgYEA4ADFJACr1uDK2aMqjR2fEz2GBm+om7j0
Omj6STyHuYxb4wlUzC58sqtNVXGTVB+ha1HZT+DM7NHkCtEQLN2dqVrFCgv39MzI
huxD59j0hsVrqnK23JswgsiJar8AUiEe2MY/ysgHVuUtkDzKEN+bgcVhqap0sn4C
q19oOcA/aqcCgYAefhcJJxtHdRu7tbgfvBEJ+WHNG+kdfhR3N6p1u1N9JHLnOohv
evLdViuoIz7s8mdPTAvqshSgjfpao7rRsHZq9ToGJzHOkN+mOofVC8vwufM0/8da
kfGbmvhQ9scuDuZAQZ4mu1/IBmeL/0POKP0hRzy43IngpmBMNzNocODWywKBgC/b
ukLw6cXlDTHmjIbN11jTAjmJzapHn9aC60aOaikYdeFR8w4UuIur0b/5nhKRF3nI
aPeJ/f5y8ZfmBuCvEKpIPGTjHbztq8I35GI6ljPdJh2qmKsVdQ3cLo/h8v2ZGfAS
mzqF9ht4p31zn3BvddgKBc2sH3arOYLHxYrhKittAoGAIpHK6rYu3O/1eF5Nimok
fRpT+FVOxgRX9fBL6IBuYzZPvtDfhq1kgzOYBo1oW+ObaF9BnFvYdE2CEX8Gr0+Z
s6psX64trGW6DcgvM8bQtQa1EfRSp/EifPGwa5tzPw0UVF/VdpFa9Lum7cAjTDsm
jKIQUja2I9E99ZWstIdBCUE=
-----END PRIVATE KEY-----"#;

    #[test]
    fn can_decrypt_pkcs8_key() {
        let key = {
            let key = pem::parse(TEST_KEY).unwrap();
            assert_eq!(key.tag, "ENCRYPTED PRIVATE KEY");
            DecryptedKey::parse_and_decrypt(&key.contents, "foobar").unwrap()
        };

        let decrypted = pem::parse(DECYPTED_KEY).unwrap();
        assert_eq!(decrypted.tag, "PRIVATE KEY");
        assert_eq!(key.as_bytes(), decrypted.contents);
    }
}
