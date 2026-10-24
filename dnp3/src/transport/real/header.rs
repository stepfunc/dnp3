// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::transport::real::constants::{FIN_MASK, FIR_MASK};
use crate::transport::real::sequence::Sequence;

#[derive(Copy, Clone)]
pub(crate) struct Header {
    pub(crate) fin: bool,
    pub(crate) fir: bool,
    pub(crate) seq: Sequence,
}

impl Header {
    pub(crate) fn new(fin: bool, fir: bool, seq: Sequence) -> Self {
        Header { fin, fir, seq }
    }

    pub(crate) fn from_u8(value: u8) -> Self {
        Self {
            fin: value & FIN_MASK != 0,
            fir: value & FIR_MASK != 0,
            seq: Sequence::new(value),
        }
    }
    pub(crate) fn to_u8(self) -> u8 {
        let mut acc: u8 = 0;

        if self.fin {
            acc |= FIN_MASK;
        }
        if self.fir {
            acc |= FIR_MASK;
        }

        acc | self.seq.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_fir_fin_and_sequence_as_on_the_wire() {
        assert_eq!(Header::new(false, true, Sequence::new(5)).to_u8(), 0x45);
        assert_eq!(Header::new(true, false, Sequence::new(63)).to_u8(), 0xBF);
        assert_eq!(Header::new(true, true, Sequence::new(0)).to_u8(), 0xC0);
        assert_eq!(Header::new(false, false, Sequence::new(42)).to_u8(), 0x2A);
    }

    #[test]
    fn reads_fir_fin_and_sequence_as_on_the_wire() {
        let fields = |x: u8| {
            let header = Header::from_u8(x);
            (header.fin, header.fir, header.seq.value())
        };
        assert_eq!(fields(0x85), (true, false, 5));
        assert_eq!(fields(0x7F), (false, true, 63));
        assert_eq!(fields(0xC0), (true, true, 0));
        assert_eq!(fields(0x2A), (false, false, 42));
    }
}
