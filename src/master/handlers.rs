use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderIterator;

pub trait ResponseHandler {
    fn handle(&mut self, source: u16, header: ResponseHeader, iterator: HeaderIterator);
}

pub struct LoggingResponseHandler;

impl LoggingResponseHandler {
    pub fn create() -> Box<dyn ResponseHandler> {
        Box::new(Self {})
    }
}

impl ResponseHandler for LoggingResponseHandler {
    fn handle(&mut self, source: u16, header: ResponseHeader, iterator: HeaderIterator) {
        log::info!(
            "response - source: {} fir: {} fin: {}",
            source,
            header.control.fir,
            header.control.fin
        );
        for x in iterator {
            log::info!("header: {}", x);
        }
    }
}
