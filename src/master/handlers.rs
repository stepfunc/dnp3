use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::fixed::{Group51Var1, Group51Var2};
use crate::app::gen::variations::ranged::RangedVariation;
use crate::app::header::ResponseHeader;
use crate::app::meas::*;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};

pub trait ResponseHandler {
    fn handle(&mut self, source: u16, header: ResponseHeader, headers: HeaderCollection);
}

pub trait MeasurementHandler {
    fn handle_binary(&mut self, x: impl Iterator<Item = (Binary, u16)>);
    fn handle_double_binary(&mut self, x: impl Iterator<Item = (DoubleBitBinary, u16)>);
    fn handle_binary_output_status(&mut self, x: impl Iterator<Item = (BinaryOutputStatus, u16)>);
    fn handle_counter(&mut self, x: impl Iterator<Item = (Counter, u16)>);
    fn handle_frozen_counter(&mut self, x: impl Iterator<Item = (FrozenCounter, u16)>);
    fn handle_analog(&mut self, x: impl Iterator<Item = (Analog, u16)>);
    fn handle_output_status(&mut self, x: impl Iterator<Item = (AnalogOutputStatus, u16)>);
}

pub fn extract_measurements<T>(headers: HeaderCollection, handler: &mut T)
where
    T: MeasurementHandler,
{
    fn extract_cto_g51v1(prev: Time, seq: CountSequence<Group51Var1>) -> Time {
        seq.iter()
            .next()
            .map_or(prev, |x| Time::Synchronized(x.time))
    }

    fn extract_cto_g51v2(prev: Time, seq: CountSequence<Group51Var2>) -> Time {
        seq.iter()
            .next()
            .map_or(prev, |x| Time::NotSynchronized(x.time))
    }

    fn handle<T>(cto: Time, header: ObjectHeader, handler: &mut T) -> Time
    where
        T: MeasurementHandler,
    {
        match header.details {
            // these are common-time-of-occurrence headers
            HeaderDetails::OneByteCount(1, CountVariation::Group51Var1(seq)) => {
                return extract_cto_g51v1(cto, seq)
            }
            HeaderDetails::OneByteCount(1, CountVariation::Group51Var2(seq)) => {
                return extract_cto_g51v2(cto, seq)
            }
            HeaderDetails::TwoByteCount(1, CountVariation::Group51Var1(seq)) => {
                return extract_cto_g51v1(cto, seq)
            }
            HeaderDetails::TwoByteCount(1, CountVariation::Group51Var2(seq)) => {
                return extract_cto_g51v2(cto, seq)
            }
            // everything else
            HeaderDetails::OneByteStartStop(_, _, RangedVariation::Group1Var2(seq)) => {
                handler.handle_binary(seq.iter().map(|(v, i)| (Binary::from(v), i)))
            }
            _ => {}
        };
        // if we didn't return early b/c the header is a CTO, then we just return the previous value
        cto
    }

    headers
        .iter()
        .fold(Time::Invalid, |cto, header| handle(cto, header, handler));
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
