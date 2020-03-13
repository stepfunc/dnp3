pub mod writer;

pub mod constants {
    pub const FIN_MASK: u8 = 0b1000_0000;
    pub const FIR_MASK: u8 = 0b0100_0000;
    pub const SEQ_MASK: u8 = 0b0011_1111;
}
