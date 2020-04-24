use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::bytes::Bytes;
use crate::app::parse::parser::HeaderCollection;
use crate::master::runner::TaskError;
use crate::master::types::CommandError;

pub trait ResponseHandler: Send {
    fn handle(&mut self, source: u16, header: ResponseHeader, headers: HeaderCollection);
}

/// A generic callback type that can only be invoked once.
/// The user can select to implement it using FnOnce or a
/// one-shot reply channel
pub enum CallbackOnce<T> {
    BoxedFn(Box<dyn FnOnce(T) -> () + Send>),
    OneShot(tokio::sync::oneshot::Sender<T>),
}

impl<T> CallbackOnce<T> {
    pub(crate) fn complete(self, value: T) {
        match self {
            CallbackOnce::BoxedFn(func) => func(value),
            CallbackOnce::OneShot(s) => {
                s.send(value).ok();
            }
        }
    }
}

pub trait RequestCompletionHandler: Send {
    fn on_complete(&mut self, result: Result<(), TaskError>);
}

pub type CommandResult = Result<(), CommandError>;
pub type CommandCallback = CallbackOnce<CommandResult>;

pub trait AssociationHandler: ResponseHandler {
    // TODO - add additional methods
}

pub trait ReadTaskHandler: ResponseHandler + RequestCompletionHandler {}

impl<T> ReadTaskHandler for T where T: ResponseHandler + RequestCompletionHandler {}

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

#[derive(Copy, Clone)]
pub struct NullHandler;

impl NullHandler {
    pub fn boxed() -> Box<NullHandler> {
        Box::new(Self {})
    }
}

impl ResponseHandler for NullHandler {
    fn handle(&mut self, _source: u16, _header: ResponseHeader, _headers: HeaderCollection) {}
}

impl AssociationHandler for NullHandler {}

impl RequestCompletionHandler for NullHandler {
    fn on_complete(&mut self, _result: Result<(), TaskError>) {}
}
