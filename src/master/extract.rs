use crate::app::gen::variations::count::CountVariation;
use crate::app::gen::variations::fixed::*;
use crate::app::measurement::*;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};
use crate::master::handlers::MeasurementHandler;

/// Extract measurements from a HeaderCollection, sinking them into
/// a something that implements `MeasurementHandler`
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
            HeaderDetails::OneByteStartStop(_, _, var) => var.extract_measurements_to(handler),
            HeaderDetails::TwoByteStartStop(_, _, var) => var.extract_measurements_to(handler),
            HeaderDetails::OneByteCountAndPrefix(_, var) => {
                var.extract_measurements_to(cto, handler)
            }
            HeaderDetails::TwoByteCountAndPrefix(_, var) => {
                var.extract_measurements_to(cto, handler)
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
