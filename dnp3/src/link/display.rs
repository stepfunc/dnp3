use crate::decode::LinkDecodeLevel;
use crate::link::header::Header;

pub(crate) struct LinkDisplay<'a> {
    header: Header,
    payload: &'a [u8],
    level: LinkDecodeLevel,
}

impl<'a> LinkDisplay<'a> {
    pub(crate) fn new(header: Header, payload: &'a [u8], level: LinkDecodeLevel) -> Self {
        LinkDisplay {
            header,
            payload,
            level,
        }
    }
}

impl<'a> std::fmt::Display for LinkDisplay<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if self.level.header_enabled() {
            write!(
                f,
                "{:?} source: {} destination: {} payload: {} bytes",
                self.header.control,
                self.header.source.value(),
                self.header.destination.value(),
                self.payload.len()
            )?;
        }
        if self.level.payload_enabled() {
            crate::util::decode::format_bytes(f, self.payload)?;
        }
        Ok(())
    }
}
