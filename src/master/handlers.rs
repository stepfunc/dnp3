use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::bytes::Bytes;
use crate::app::parse::parser::HeaderCollection;

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
    fn handle_octet_string<'a>(&mut self, x: impl Iterator<Item = (Bytes<'a>, u16)>);
}

pub struct NullResponseHandler;

impl NullResponseHandler {
    pub fn create() -> Box<dyn ResponseHandler> {
        Box::new(Self {})
    }
}

impl ResponseHandler for NullResponseHandler {
    fn handle(&mut self, _source: u16, _header: ResponseHeader, _headers: HeaderCollection) {}
}
