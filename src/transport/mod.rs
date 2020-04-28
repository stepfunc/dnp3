use crate::link::header::Address;

#[cfg(not(test))]
/// This type definition is used so that we can mock the transport reader during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type ReaderType = crate::transport::reader::Reader;
#[cfg(not(test))]
/// This type definition is used so that we can mock the transport writer during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub(crate) type WriterType = crate::transport::writer::Writer;

#[cfg(test)]
pub(crate) mod mocks;
#[cfg(test)]
pub(crate) type ReaderType = crate::transport::mocks::MockReader;
#[cfg(test)]
pub(crate) type WriterType = crate::transport::mocks::MockWriter;

pub(crate) mod assembler;
pub(crate) mod header;
pub(crate) mod reader;
pub(crate) mod sequence;
pub(crate) mod writer;

#[derive(Debug)]
pub(crate) struct Fragment<'a> {
    pub(crate) address: Address,
    pub(crate) data: &'a [u8],
}

pub(crate) mod constants {
    pub(crate) const FIN_MASK: u8 = 0b1000_0000;
    pub(crate) const FIR_MASK: u8 = 0b0100_0000;
}

pub(crate) fn create_transport_layer(is_master: bool, address: u16) -> (ReaderType, WriterType) {
    (
        ReaderType::new(is_master, address),
        WriterType::new(is_master, address),
    )
}
