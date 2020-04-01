use crate::transport::reader::Reader;
use crate::transport::writer::Writer;

#[cfg(not(test))]
pub type ReaderType = crate::transport::reader::Reader;
#[cfg(not(test))]
pub type WriterType = crate::transport::writer::Writer;

#[cfg(test)]
pub mod mocks;
#[cfg(test)]
pub type ReaderType = crate::transport::mocks::MockReader;
#[cfg(test)]
pub type WriterType = crate::transport::mocks::MockWriter;

pub mod header;
pub mod reader;
pub mod sequence;
pub mod writer;

pub mod constants {
    pub const FIN_MASK: u8 = 0b1000_0000;
    pub const FIR_MASK: u8 = 0b0100_0000;
    pub const SEQ_MASK: u8 = 0b0011_1111;
}

pub fn create_transport_layer(is_master: bool, address: u16) -> (ReaderType, WriterType) {
    (
        ReaderType::new(is_master, address),
        WriterType::new(is_master, address),
    )
}
