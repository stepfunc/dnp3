mod crc;
pub(crate) mod formatter;
mod function;
pub(crate) mod header;
pub(crate) mod layer;
pub(crate) mod parser;
pub(crate) mod reader;

pub(crate) mod constant {
    pub(crate) const START1: u8 = 0x05;
    pub(crate) const START2: u8 = 0x64;

    pub(crate) const MAX_FRAME_PAYLOAD_LENGTH: usize = 250;
    pub(crate) const MAX_LINK_FRAME_LENGTH: usize = 292;
    pub(crate) const MAX_APP_BYTES_PER_FRAME: usize = MAX_FRAME_PAYLOAD_LENGTH - 1;
    pub(crate) const MIN_HEADER_LENGTH_VALUE: u8 = 5;
    pub(crate) const MAX_BLOCK_SIZE: usize = 16;
    pub(crate) const CRC_LENGTH: usize = 2;
    pub(crate) const MAX_BLOCK_SIZE_WITH_CRC: usize = MAX_BLOCK_SIZE + CRC_LENGTH;
}

#[cfg(test)]
pub(crate) mod test_data {
    use crate::link::function::Function;
    use crate::link::header::{Address, ControlField, Header};

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
            address: Address {
                destination: 1,
                source: 1024,
            },
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
            address: Address {
                destination: 1024,
                source: 1,
            },
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
            address: Address {
                destination: 1,
                source: 1024,
            },
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
            address: Address {
                destination: 1,
                source: 1024,
            },
        },
        payload: &[
            0xC0, 0xC5, 0x02, 0x32, 0x01, 0x07, 0x01, 0xF8, 0xB8, 0x6C, 0xAA, 0xF0, 0x00,
        ],
    };
}
