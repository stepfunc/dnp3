use crate::app::header::ResponseHeader;
use crate::app::meas::*;
use crate::app::parse::bytes::RangedBytesSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails};

pub trait ResponseHandler {
    fn handle(&mut self, source: u16, header: ResponseHeader, headers: HeaderCollection);
}

pub trait MeasurementHandler {
    fn handle_binary(&mut self, x: impl Iterator<Item = (Binary, u16)>);
    fn handle_double_bit_binary(&mut self, x: impl Iterator<Item = (DoubleBitBinary, u16)>);
    fn handle_binary_output_status(&mut self, x: impl Iterator<Item = (BinaryOutputStatus, u16)>);
    fn handle_counter(&mut self, x: impl Iterator<Item = (Counter, u16)>);
    fn handle_frozen_counter(&mut self, x: impl Iterator<Item = (FrozenCounter, u16)>);
    fn handle_analog(&mut self, x: impl Iterator<Item = (Analog, u16)>);
    fn handle_analog_output_status(&mut self, x: impl Iterator<Item = (AnalogOutputStatus, u16)>);
    fn handle_octet_string(&mut self, x: &RangedBytesSequence);
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
