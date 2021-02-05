use crate::config::TransportDecodeLevel;
use crate::transport::real::header::Header;

use std::fmt::Write;

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

impl<'a> std::fmt::Display for SegmentDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.level.header_enabled() {
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
            for chunk in self.payload.chunks(16) {
                writeln!(f)?;
                let mut first = true;
                for byte in chunk {
                    if !first {
                        f.write_char(' ')?;
                    }
                    first = false;
                    write!(f, "{:02X?}", byte)?;
                }
            }
        }
        Ok(())
    }
}
