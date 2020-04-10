use crate::transport::sequence::Sequence;

#[derive(Copy, Clone)]
pub(crate) struct Header {
    pub(crate) fin: bool,
    pub(crate) fir: bool,
    pub(crate) seq: Sequence,
}

impl Header {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            fin: value & super::constants::FIN_MASK != 0,
            fir: value & super::constants::FIR_MASK != 0,
            seq: Sequence::new(value),
        }
    }
}
