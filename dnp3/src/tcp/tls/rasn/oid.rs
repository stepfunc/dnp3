pub(crate) enum AlgorithmID {
    Ed25519,
    SHA1WithRSASignature,
    RSAEncryption,
}

impl AlgorithmID {
    pub(crate) fn to_str(&self) -> &str {
        match self {
            AlgorithmID::Ed25519 => "Ed25519 Signature",
            AlgorithmID::SHA1WithRSASignature => "SHA1 with RSA Signature",
            AlgorithmID::RSAEncryption => "RSA Encryption",
        }
    }
}

pub(crate) enum KnownOID {
    CommonName,
    OrganizationName,
    OrganizationalUnitName,
    CountryName,
    StateOrProvinceName,
    LocalityName,
    EmailAddress,
    Algorithm(AlgorithmID),
}

impl KnownOID {
    pub(crate) fn to_str(&self) -> &str {
        match self {
            KnownOID::CommonName => "Common Name",
            KnownOID::CountryName => "Country Name",
            KnownOID::OrganizationName => "Organization Name",
            KnownOID::OrganizationalUnitName => "Organizational Unit Name",
            KnownOID::StateOrProvinceName => "State or Province Name",
            KnownOID::LocalityName => "Locality Name",
            KnownOID::EmailAddress => "Email Address",
            KnownOID::Algorithm(id) => id.to_str(),
        }
    }
}

pub(crate) fn get_oid(id: &[u32]) -> Option<KnownOID> {
    match id {
        [1, 2, 840, 113_549, 1, 1, 1] => Some(KnownOID::Algorithm(AlgorithmID::RSAEncryption)),
        [1, 2, 840, 113_549, 1, 1, 5] => {
            Some(KnownOID::Algorithm(AlgorithmID::SHA1WithRSASignature))
        }
        [1, 3, 101, 112] => Some(KnownOID::Algorithm(AlgorithmID::Ed25519)),
        [2, 5, 4, 3] => Some(KnownOID::CommonName),
        [2, 5, 4, 6] => Some(KnownOID::CountryName),
        [2, 5, 4, 7] => Some(KnownOID::LocalityName),
        [2, 5, 4, 10] => Some(KnownOID::OrganizationName),
        [2, 5, 4, 11] => Some(KnownOID::OrganizationalUnitName),
        [2, 5, 4, 8] => Some(KnownOID::StateOrProvinceName),
        [1, 2, 840, 113_549, 1, 9, 1] => Some(KnownOID::EmailAddress),

        _ => None,
    }
}
