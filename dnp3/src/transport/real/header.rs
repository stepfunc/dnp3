use crate::transport::constants::{FIN_MASK, FIR_MASK};
use crate::transport::real::sequence::Sequence;

#[derive(Copy, Clone)]
pub(crate) struct Header {
    pub(crate) fin: bool,
    pub(crate) fir: bool,
    pub(crate) seq: Sequence,
}

impl Header {
    pub(crate) fn new(value: u8) -> Self {
        Self {
            fin: value & FIN_MASK != 0,
            fir: value & FIR_MASK != 0,
            seq: Sequence::new(value),
        }
    }
}
