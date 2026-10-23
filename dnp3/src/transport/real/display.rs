// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::decode::TransportDecodeLevel;
use crate::transport::real::header::Header;

pub(crate) struct SegmentDisplay<'a> {
    pub(crate) header: Header,
    pub(crate) payload: &'a [u8],
    level: TransportDecodeLevel,
}

impl<'a> SegmentDisplay<'a> {
    pub(crate) fn new(header: Header, payload: &'a [u8], level: TransportDecodeLevel) -> Self {
        Self {
            header,
            payload,
            level,
        }
    }
}

impl std::fmt::Display for SegmentDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.level.enabled() {
            write!(
                f,
                "FIN: {} FIR: {} SEQ: {} length: {}",
                self.header.fin,
                self.header.fir,
                self.header.seq.value(),
                self.payload.len()
            )?;
        }
        if self.level.payload_enabled() {
            crate::util::decode::format_bytes(f, self.payload)?;
        }
        Ok(())
    }
}
