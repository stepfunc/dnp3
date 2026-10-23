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
