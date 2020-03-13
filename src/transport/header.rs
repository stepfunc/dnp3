#[derive(Copy, Clone)]
pub struct Header {
    pub fin: bool,
    pub fir: bool,
    pub seq: u8,
}

impl Header {
    pub fn new(value: u8) -> Self {
        Self {
            fin: value & super::constants::FIN_MASK != 0,
            fir: value & super::constants::FIR_MASK != 0,
            seq: value & super::constants::SEQ_MASK,
        }
    }
}
