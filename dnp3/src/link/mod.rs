use crate::link::header::AnyAddress;

mod crc;
pub(crate) mod display;
pub(crate) mod error;
pub(crate) mod format;
mod function;
pub(crate) mod header;
pub(crate) mod layer;
pub(crate) mod parser;
pub(crate) mod reader;

pub(crate) mod constant {
    pub(crate) const START1: u8 = 0x05;
    pub(crate) const START2: u8 = 0x64;

    pub(crate) const MAX_FRAME_PAYLOAD_LENGTH: usize = 250;
    pub(crate) const LINK_HEADER_LENGTH: usize = 10;
    pub(crate) const MAX_LINK_FRAME_LENGTH: usize = 292;
    pub(crate) const MAX_APP_BYTES_PER_FRAME: usize = MAX_FRAME_PAYLOAD_LENGTH - 1;
    pub(crate) const MIN_HEADER_LENGTH_VALUE: u8 = 5;
    pub(crate) const MAX_BLOCK_SIZE: usize = 16;
    pub(crate) const CRC_LENGTH: usize = 2;
    pub(crate) const MAX_BLOCK_SIZE_WITH_CRC: usize = MAX_BLOCK_SIZE + CRC_LENGTH;
}

/// Controls how errors in parsed link-layer frames are handled. This behavior
/// is configurable for physical layers with built-in error correction like TCP
/// as the connection might be through a terminal server.
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum LinkErrorMode {
    /// Framing errors are discarded. The link-layer parser is reset on any error, and the
    /// parser begins scanning for 0x0564. This is always the behavior for serial ports.
    Discard,
    /// Framing errors are bubbled up to calling code, closing the session. Suitable for physical
    /// layers that provide error correction like TCP.
    Close,
}

/// Controls how the link-layer parser treats frames that span multiple calls to read of
/// the physical layer.
///
/// UDP is unique in that the specification requires that link layer frames be wholly contained
/// within datagrams, but this can be relaxed by configuration.
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LinkReadMode {
    /// Reading from a stream (TCP, serial, etc.) where link-layer frames MAY span separate calls to read
    Stream,
    /// Reading datagrams (UDP) where link-layer frames MAY NOT span separate calls to read
    Datagram,
}

/// Represents a validated 16-bit endpoint address for a master or an outstation
/// Certain special addresses are not allowed by the standard to be used
/// as endpoint addresses.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "serialization", serde(try_from = "u16"))]
pub struct EndpointAddress(u16);

/// The specified address is special and may not be used as an EndpointAddress
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SpecialAddressError {
    /// reserved special address
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
    pub fn try_new(value: u16) -> Result<Self, SpecialAddressError> {
        value.try_into()
    }

    /// get the raw u16 value of the address
    pub fn raw_value(&self) -> u16 {
        self.0
    }

    pub(crate) const fn raw(address: u16) -> EndpointAddress {
        EndpointAddress(address)
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
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
pub(crate) mod test_data {
    use crate::link::function::Function;
    use crate::link::header::{AnyAddress, ControlField, Header};

    pub(crate) struct TestFrame {
        pub(crate) bytes: &'static [u8],
        pub(crate) header: Header,
        pub(crate) payload: &'static [u8],
    }

    pub(crate) const RESET_LINK: TestFrame = TestFrame {
        bytes: &[0x05, 0x64, 0x05, 0xC0, 0x01, 0x00, 0x00, 0x04, 0xE9, 0x21],
        header: Header {
            control: ControlField {
                func: Function::PriResetLinkStates,
                master: true,
                fcb: false,
                fcv: false,
            },
            destination: AnyAddress::from(1),
            source: AnyAddress::from(1024),
        },
        payload: &[],
    };

    pub(crate) const ACK: TestFrame = TestFrame {
        bytes: &[0x05, 0x64, 0x05, 0x00, 0x00, 0x04, 0x01, 0x00, 0x19, 0xA6],
        header: Header {
            control: ControlField {
                func: Function::SecAck,
                master: false,
                fcb: false,
                fcv: false,
            },
            destination: AnyAddress::from(1024),
            source: AnyAddress::from(1),
        },
        payload: &[],
    };

    pub(crate) const CONFIRM_USER_DATA: TestFrame = TestFrame {
        bytes: &[
            // header
            0x05, 0x64, 0x14, 0xF3, 0x01, 0x00, 0x00, 0x04, 0x0A, 0x3B, // body
            0xC0, 0xC3, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01,
            0x06, 0x9A, 0x12,
        ],
        header: Header {
            control: ControlField {
                func: Function::PriConfirmedUserData,
                master: true,
                fcb: true,
                fcv: true,
            },
            destination: AnyAddress::from(1),
            source: AnyAddress::from(1024),
        },
        payload: &[
            0xC0, 0xC3, 0x01, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06, 0x3C, 0x01,
            0x06,
        ],
    };

    pub(crate) const UNCONFIRMED_USER_DATA: TestFrame = TestFrame {
        bytes: &[
            0x05, 0x64, 0x12, 0xC4, 0x01, 0x00, 0x00, 0x04, 0x0E, 0x0B, 0xC0, 0xC5, 0x02, 0x32,
            0x01, 0x07, 0x01, 0xF8, 0xB8, 0x6C, 0xAA, 0xF0, 0x00, 0x98, 0x98,
        ],
        header: Header {
            control: ControlField {
                func: Function::PriUnconfirmedUserData,
                master: true,
                fcb: false,
                fcv: false,
            },
            destination: AnyAddress::from(1),
            source: AnyAddress::from(1024),
        },
        payload: &[
            0xC0, 0xC5, 0x02, 0x32, 0x01, 0x07, 0x01, 0xF8, 0xB8, 0x6C, 0xAA, 0xF0, 0x00,
        ],
    };
}
