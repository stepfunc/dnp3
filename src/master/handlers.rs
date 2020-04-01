use crate::app::gen::variations::ranged::RangedVariation;
use crate::app::header::ResponseHeader;

pub trait ResponseHandler {
    fn begin(&mut self, source: u16, header: ResponseHeader);
    fn handle_ranged(&mut self, variation: RangedVariation);
    fn end(&mut self, source: u16, header: ResponseHeader);
}

pub struct LoggingResponseHandler;
impl ResponseHandler for LoggingResponseHandler {
    fn begin(&mut self, source: u16, header: ResponseHeader) {
        log::info!(
            "begin - source: {} fir: {} fin: {} con: {} uns: {} seq: {} ",
            source,
            header.control.fir,
            header.control.fin,
            header.control.con,
            header.control.uns,
            header.control.seq.value(),
        );
    }

    fn handle_ranged(&mut self, variation: RangedVariation) {
        match variation {
            RangedVariation::Group1Var2(seq) => {
                for (v, i) in seq.iter() {
                    log::info!("index: {} flags: {}", i, v.flags);
                }
            }
            _ => log::info!("unknown ranged variation"),
        }
    }

    fn end(&mut self, source: u16, header: ResponseHeader) {
        log::info!(
            "end - source: {} fir: {} fin: {} con: {} uns: {} seq: {} ",
            source,
            header.control.fir,
            header.control.fin,
            header.control.con,
            header.control.uns,
            header.control.seq.value(),
        );
    }
}
