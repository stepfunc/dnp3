use crate::transport::reader::Reader;
use crate::transport::writer::Writer;

pub mod header;
pub mod reader;
pub mod sequence;
pub mod writer;

pub mod constants {
    pub const FIN_MASK: u8 = 0b1000_0000;
    pub const FIR_MASK: u8 = 0b0100_0000;
    pub const SEQ_MASK: u8 = 0b0011_1111;
}

pub fn create_transport_layer(is_master: bool, address: u16) -> (Reader, Writer) {
    (
        Reader::new(is_master, address),
        Writer::new(is_master, address),
    )
}
