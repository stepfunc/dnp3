use crate::app::header::ResponseHeader;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails};

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
            match x.details {
                HeaderDetails::AllObjects(_) => {}
                HeaderDetails::OneByteStartStop(_, _, var) => var.log(log::Level::Info),
                HeaderDetails::TwoByteStartStop(_, _, var) => var.log(log::Level::Info),
                HeaderDetails::OneByteCount(_, _var) => {}
                HeaderDetails::TwoByteCount(_, _var) => {}
                HeaderDetails::OneByteCountAndPrefix(_, _var) => {}
                HeaderDetails::TwoByteCountAndPrefix(_, _var) => {}
            }
        }
    }
}
