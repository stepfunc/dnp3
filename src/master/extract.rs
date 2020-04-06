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

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::flags::Flags;
    use crate::app::gen::enums::FunctionCode;
    use crate::app::parse::bytes::Bytes;
    use crate::app::parse::parser::HeaderCollection;
    use crate::app::types::Timestamp;
    use crate::master::handlers::MeasurementHandler;

    #[derive(Debug)]
    enum Header {
        Binary(Vec<(Binary, u16)>),
    }

    struct MockHandler {
        expected: Vec<Header>,
    }

    impl MockHandler {
        fn new() -> Self {
            Self { expected: vec![] }
        }

        fn is_empty(&self) -> bool {
            self.expected.is_empty()
        }

        fn expect(&mut self, header: Header) {
            self.expected.push(header)
        }
    }

    impl MeasurementHandler for MockHandler {
        fn handle_binary(&mut self, x: impl Iterator<Item = (Binary, u16)>) {
            let next_header = match self.expected.pop() {
                Some(y) => y,
                None => {
                    panic!("Not expecting any headers!");
                }
            };

            match next_header {
                Header::Binary(expected) => {
                    let received: Vec<_> = x.collect();
                    assert_eq!(received, expected);
                } //x => panic!("Unexpected header: {:?}", x)
            }
        }

        fn handle_double_bit_binary(&mut self, _x: impl Iterator<Item = (DoubleBitBinary, u16)>) {
            unimplemented!()
        }

        fn handle_binary_output_status(
            &mut self,
            _x: impl Iterator<Item = (BinaryOutputStatus, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_counter(&mut self, _x: impl Iterator<Item = (Counter, u16)>) {
            unimplemented!()
        }

        fn handle_frozen_counter(&mut self, _x: impl Iterator<Item = (FrozenCounter, u16)>) {
            unimplemented!()
        }

        fn handle_analog(&mut self, _x: impl Iterator<Item = (Analog, u16)>) {
            unimplemented!()
        }

        fn handle_analog_output_status(
            &mut self,
            _x: impl Iterator<Item = (AnalogOutputStatus, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_octet_string<'a>(&mut self, _x: impl Iterator<Item = (Bytes<'a>, u16)>) {
            unimplemented!()
        }
    }

    #[test]
    fn g2v3_without_cto_yields_invalid_time() {
        let mut handler = MockHandler::new();
        let headers = HeaderCollection::parse(
            FunctionCode::Response,
            &[0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF],
        )
        .unwrap();

        let expected: (Binary, u16) = (
            Binary {
                value: false,
                flags: Flags::ONLINE,
                time: Time::Invalid,
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(headers, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn g2v3_with_synchronized_cto_yields_synchronized_time() {
        let mut handler = MockHandler::new();
        let headers = HeaderCollection::parse(
            FunctionCode::Response,
            // g50v1       count: 1   -------- time: 1 ------------------
            &[
                0x33, 0x01, 0x07, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03, 0x17, 0x01,
                0x07, 0x01, 0xFF, 0xFF,
            ],
        )
        .unwrap();

        let expected: (Binary, u16) = (
            Binary {
                value: false,
                flags: Flags::ONLINE,
                time: Time::Synchronized(Timestamp::new(65537)), // 0xFFFF + 2
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(headers, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn g2v3_with_unsynchronized_cto_yields_unsynchronized_time() {
        let mut handler = MockHandler::new();
        let headers = HeaderCollection::parse(
            FunctionCode::Response,
            // g50v2   count: 1    --------- time: 2 -----------------
            &[
                0x33, 0x02, 0x07, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x03, 0x17, 0x01,
                0x07, 0x01, 0xFE, 0xFF,
            ],
        )
        .unwrap();

        let expected: (Binary, u16) = (
            Binary {
                value: false,
                flags: Flags::ONLINE,
                time: Time::NotSynchronized(Timestamp::new(65536)), // 0xFFFE + 2
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(headers, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn can_calculate_maximum_timestamp() {
        let mut handler = MockHandler::new();
        let headers = HeaderCollection::parse(
            FunctionCode::Response,
            // g50v1      count: 1  --------- time: 0xFFFFFE ----------
            &[
                0x33, 0x01, 0x07, 0x01, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0x03, 0x17, 0x01,
                0x07, 0x01, 0x01, 0x00,
            ],
        )
        .unwrap();

        let expected: (Binary, u16) = (
            Binary {
                value: false,
                flags: Flags::ONLINE,
                time: Time::Synchronized(Timestamp::max()),
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(headers, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn cto_overflow_of_u48_yields_invalid_time() {
        let mut handler = MockHandler::new();
        let headers = HeaderCollection::parse(
            FunctionCode::Response,
            // g50v1      count: 1  --------- time: 0xFFFFFE ----------
            &[
                0x33, 0x01, 0x07, 0x01, 0xFE, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x02, 0x03, 0x17, 0x01,
                0x07, 0x01, 0x02, 0x00,
            ],
        )
        .unwrap();

        let expected: (Binary, u16) = (
            Binary {
                value: false,
                flags: Flags::ONLINE,
                time: Time::Invalid,
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(headers, &mut handler);
        assert!(handler.is_empty());
    }
}
