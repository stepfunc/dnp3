use crate::app::header::ResponseHeader;
use crate::app::measurement::{
    Analog, AnalogOutputStatus, Binary, BinaryOutputStatus, Counter, DoubleBitBinary, FrozenCounter,
};
use crate::app::parse::bytes::Bytes;
use crate::master::handle::{AssociationHandler, ReadHandler};
use std::time::SystemTime;

#[derive(Copy, Clone)]
pub struct NullHandler;

impl NullHandler {
    pub fn boxed() -> Box<NullHandler> {
        Box::new(Self {})
    }
}

impl ReadHandler for NullHandler {
    fn begin_fragment(&mut self, _header: ResponseHeader) {}

    fn end_fragment(&mut self, _header: ResponseHeader) {}

    fn handle_binary(&mut self, _iter: &mut dyn Iterator<Item = (Binary, u16)>) {}

    fn handle_double_bit_binary(
        &mut self,
        _iter: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
    ) {
    }

    fn handle_binary_output_status(
        &mut self,
        _iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
    }

    fn handle_counter(&mut self, _iter: &mut dyn Iterator<Item = (Counter, u16)>) {}

    fn handle_frozen_counter(&mut self, _iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>) {}

    fn handle_analog(&mut self, _iter: &mut dyn Iterator<Item = (Analog, u16)>) {}

    fn handle_analog_output_status(
        &mut self,
        _iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
    }

    fn handle_octet_string<'a>(&mut self, _iter: &mut dyn Iterator<Item = (Bytes<'a>, u16)>) {}
}

impl AssociationHandler for NullHandler {
    fn get_system_time(&self) -> SystemTime {
        SystemTime::now()
    }

    fn get_read_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }
}
