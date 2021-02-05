use crate::link::header::AnyAddress;
use std::convert::{TryFrom, TryInto};

/// Controls how errors in parsed link-layer frames are handled. This behavior
/// is configurable for physical layers with built-in error correction like TCP
/// as the connection might be through a terminal server.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum LinkErrorMode {
    /// Framing errors are discarded. The link-layer parser is reset on any error, and the
    /// parser begins scanning for 0x0564. This is always the behavior for serial ports.
    Discard,
    /// Framing errors are bubbled up to calling code, closing the session. Suitable for physical
    /// layers that provide error correction like TCP.
    Close,
}

/// Controls the decoding of transmitted and received data at the application, transport, and link layer
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct DecodeLevel {
    /// Controls application layer decoding
    pub application: AppDecodeLevel,
    /// Controls transport layer decoding
    pub transport: TransportDecodeLevel,
    /// Controls link layer decoding
    pub link: LinkDecodeLevel,
}

impl DecodeLevel {
    pub fn nothing() -> Self {
        Self::default()
    }

    pub fn new(
        application: AppDecodeLevel,
        transport: TransportDecodeLevel,
        link: LinkDecodeLevel,
    ) -> Self {
        DecodeLevel {
            application,
            transport,
            link,
        }
    }
}

impl Default for DecodeLevel {
    fn default() -> Self {
        Self {
            application: AppDecodeLevel::Nothing,
            transport: TransportDecodeLevel::Nothing,
            link: LinkDecodeLevel::Nothing,
        }
    }
}

/// Controls how transmitted and received application-layer fragments are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AppDecodeLevel {
    /// Decode nothing
    Nothing,
    /// Decode the header-only
    Header,
    /// Decode the header and the object headers
    ObjectHeaders,
    /// Decode the header, the object headers, and the object values
    ObjectValues,
}

/// Controls how transmitted and received transport segments are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum TransportDecodeLevel {
    /// Decode nothing
    Nothing,
    /// Decode the header
    Header,
    /// Decode the header and the raw payload as hexadecimal
    Payload,
}

/// Controls how transmitted and received link frames are decoded at the INFO log level
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum LinkDecodeLevel {
    /// Decode nothing
    Nothing,
    /// Decode the header
    Header,
    /// Decode the header and the raw payload as hexadecimal
    Payload,
}

/// Represents a validated 16-bit endpoint address for a master or an outstation
/// Certain special addresses are not allowed by the standard to be used
/// as endpoint addresses.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct EndpointAddress {
    address: u16,
}

/// The specified address is special and may not be used as an EndpointAddress
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct SpecialAddressError {
    pub address: u16,
}

impl std::error::Error for SpecialAddressError {}

impl std::fmt::Display for SpecialAddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "special address ({}) may not be used as a master or outstation address",
            self.address
        )
    }
}

impl EndpointAddress {
    /// try to construct an EndpointAddress from a raw u16
    pub fn from(value: u16) -> Result<Self, SpecialAddressError> {
        value.try_into()
    }

    /// get the raw u16 value of the address
    pub fn raw_value(&self) -> u16 {
        self.address
    }

    pub(crate) const fn raw(address: u16) -> EndpointAddress {
        EndpointAddress { address }
    }

    pub(crate) fn wrap(&self) -> AnyAddress {
        AnyAddress::Endpoint(*self)
    }
}

impl TryFrom<u16> for EndpointAddress {
    type Error = SpecialAddressError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        match AnyAddress::from(value) {
            AnyAddress::Endpoint(x) => Ok(x),
            _ => Err(SpecialAddressError { address: value }),
        }
    }
}

impl std::fmt::Display for EndpointAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.address)
    }
}

impl From<AppDecodeLevel> for DecodeLevel {
    fn from(application: AppDecodeLevel) -> Self {
        Self {
            application,
            transport: TransportDecodeLevel::Nothing,
            link: LinkDecodeLevel::Nothing,
        }
    }
}

impl AppDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
        self.header()
    }

    pub(crate) fn header(&self) -> bool {
        match self {
            AppDecodeLevel::Nothing => false,
            AppDecodeLevel::Header => true,
            AppDecodeLevel::ObjectHeaders => true,
            AppDecodeLevel::ObjectValues => true,
        }
    }

    pub(crate) fn object_headers(&self) -> bool {
        match self {
            AppDecodeLevel::Nothing => false,
            AppDecodeLevel::Header => false,
            AppDecodeLevel::ObjectHeaders => true,
            AppDecodeLevel::ObjectValues => true,
        }
    }

    pub(crate) fn object_values(&self) -> bool {
        match self {
            AppDecodeLevel::Nothing => false,
            AppDecodeLevel::Header => false,
            AppDecodeLevel::ObjectHeaders => false,
            AppDecodeLevel::ObjectValues => true,
        }
    }
}

impl TransportDecodeLevel {
    pub(crate) fn enabled(&self) -> bool {
        self.header_enabled()
    }

    pub(crate) fn header_enabled(&self) -> bool {
        match self {
            TransportDecodeLevel::Nothing => false,
            TransportDecodeLevel::Header => true,
            TransportDecodeLevel::Payload => true,
        }
    }

    pub(crate) fn payload_enabled(&self) -> bool {
        match self {
            TransportDecodeLevel::Nothing => false,
            TransportDecodeLevel::Header => false,
            TransportDecodeLevel::Payload => true,
        }
    }
}
