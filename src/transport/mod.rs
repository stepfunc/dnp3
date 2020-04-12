#[cfg(not(test))]
/// This type definition is used so that we can mock the transport reader during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub type ReaderType = crate::transport::reader::Reader;
#[cfg(not(test))]
/// This type definition is used so that we can mock the transport writer during testing.
/// If Rust eventually allows `async fn` in traits, this can be removed
pub type WriterType = crate::transport::writer::Writer;

#[cfg(test)]
pub mod mocks;
#[cfg(test)]
pub type ReaderType = crate::transport::mocks::MockReader;
#[cfg(test)]
pub type WriterType = crate::transport::mocks::MockWriter;

pub(crate) mod assembler;
pub(crate) mod header;
pub mod reader;
pub(crate) mod sequence;
pub mod writer;

pub(crate) mod constants {
    pub(crate) const FIN_MASK: u8 = 0b1000_0000;
    pub(crate) const FIR_MASK: u8 = 0b0100_0000;
}

pub fn create_transport_layer(is_master: bool, address: u16) -> (ReaderType, WriterType) {
    (
        ReaderType::new(is_master, address),
        WriterType::new(is_master, address),
    )
}
