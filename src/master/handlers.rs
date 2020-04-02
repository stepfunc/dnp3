use crate::app::header::ResponseHeader;
use crate::app::parse::parser::HeaderCollection;

pub trait ResponseHandler {
    fn handle(&mut self, source: u16, header: ResponseHeader, headers: HeaderCollection);
}

pub struct LoggingResponseHandler;

impl LoggingResponseHandler {
    pub fn create() -> Box<dyn ResponseHandler> {
        Box::new(Self {})
    }
}

impl ResponseHandler for LoggingResponseHandler {
    fn handle(&mut self, source: u16, header: ResponseHeader, headers: HeaderCollection) {
        log::info!(
            "response - source: {} fir: {} fin: {}",
            source,
            header.control.fir,
            header.control.fin
        );
        for x in headers.iter() {
            log::info!("header: {}", x);
        }
    }
}
