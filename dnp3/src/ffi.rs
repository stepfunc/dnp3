// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Internal bridge for `dnp3-ffi`. Not part of the public Rust API.

/// Classification of [`crate::tcp::tls::TlsError`] for the FFI bindings.
#[cfg(feature = "enable-tls")]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TlsErrorClass {
    /// The configured DNS name is invalid.
    InvalidDnsName,
    /// Any other TLS configuration error.
    Other,
}

#[cfg(feature = "enable-tls")]
impl From<crate::tcp::tls::TlsError> for TlsErrorClass {
    fn from(err: crate::tcp::tls::TlsError) -> Self {
        match err.0 {
            crate::tcp::tls::TlsErrorKind::InvalidDnsName => Self::InvalidDnsName,
            crate::tcp::tls::TlsErrorKind::Config(_) => Self::Other,
        }
    }
}
