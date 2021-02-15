use crate::app::gen::count::CountVariation;
use crate::app::header::ResponseHeader;
use crate::app::measurement::*;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};
use crate::app::variations::*;
use crate::master::handle::ReadHandler;

/// Extract measurements from a HeaderCollection, sinking them into
/// something that implements `MeasurementHandler`
pub(crate) fn extract_measurements(
    header: ResponseHeader,
    objects: HeaderCollection,
    handler: &mut dyn ReadHandler,
) {
    fn extract_cto_g51v1(prev: Option<Time>, item: Option<Group51Var1>) -> Option<Time> {
        item.map_or(prev, |x| Some(Time::Synchronized(x.time)))
    }

    fn extract_cto_g51v2(prev: Option<Time>, item: Option<Group51Var2>) -> Option<Time> {
        item.map_or(prev, |x| Some(Time::NotSynchronized(x.time)))
    }

    fn handle(
        cto: Option<Time>,
        header: ObjectHeader,
        handler: &mut dyn ReadHandler,
    ) -> Option<Time> {
        let handled = match &header.details {
            // these are common-time-of-occurrence headers
            HeaderDetails::OneByteCount(1, CountVariation::Group51Var1(seq)) => {
                return extract_cto_g51v1(cto, seq.single())
            }
            HeaderDetails::OneByteCount(1, CountVariation::Group51Var2(seq)) => {
                return extract_cto_g51v2(cto, seq.single())
            }
            HeaderDetails::TwoByteCount(1, CountVariation::Group51Var1(seq)) => {
                return extract_cto_g51v1(cto, seq.single())
            }
            HeaderDetails::TwoByteCount(1, CountVariation::Group51Var2(seq)) => {
                return extract_cto_g51v2(cto, seq.single())
            }
            // everything else
            HeaderDetails::OneByteStartStop(_, _, var) => {
                var.extract_measurements_to(header.details.qualifier(), handler)
            }
            HeaderDetails::TwoByteStartStop(_, _, var) => {
                var.extract_measurements_to(header.details.qualifier(), handler)
            }
            HeaderDetails::OneByteCountAndPrefix(_, var) => {
                var.extract_measurements_to(cto, handler)
            }
            HeaderDetails::TwoByteCountAndPrefix(_, var) => {
                var.extract_measurements_to(cto, handler)
            }
            _ => false,
        };

        if !handled {
            tracing::warn!(
                "Ignored header variation: {} qualifier: {:?}",
                &header.variation,
                &header.details.qualifier()
            );
        }

        // if we didn't return early b/c the header is a CTO, then we just return the previous value
        cto
    }

    handler.begin_fragment(header);
    objects
        .iter()
        .fold(None, |cto, header| handle(cto, header, handler));
    handler.end_fragment(header);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::app::header::{Control, Iin, ResponseFunction, ResponseHeader};
    use crate::app::parse::parser::HeaderCollection;
    use crate::app::Bytes;
    use crate::app::Flags;
    use crate::app::FunctionCode;
    use crate::app::Timestamp;
    use crate::master::handle::{HeaderInfo, ReadHandler};

    fn header() -> ResponseHeader {
        ResponseHeader::new(
            Control::from(0xC0),
            ResponseFunction::Response,
            Iin::default(),
        )
    }

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

    impl ReadHandler for MockHandler {
        fn begin_fragment(&mut self, _header: ResponseHeader) {}
        fn end_fragment(&mut self, _header: ResponseHeader) {}

        fn handle_binary(&mut self, _info: HeaderInfo, x: &mut dyn Iterator<Item = (Binary, u16)>) {
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

        fn handle_double_bit_binary(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (DoubleBitBinary, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_binary_output_status(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_counter(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (Counter, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_frozen_counter(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_analog(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (Analog, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_analog_output_status(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_octet_string<'a>(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (Bytes<'a>, u16)>,
        ) {
            unimplemented!()
        }
    }

    #[test]
    fn g2v3_without_cto_yields_invalid_time() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::Response,
            &[0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF],
        )
        .unwrap();

        let expected: (Binary, u16) = (
            Binary {
                value: false,
                flags: Flags::ONLINE,
                time: None,
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(header(), objects, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn g2v3_with_synchronized_cto_yields_synchronized_time() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
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
                time: Some(Time::Synchronized(Timestamp::new(65537))), // 0xFFFF + 2
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(header(), objects, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn g2v3_with_unsynchronized_cto_yields_unsynchronized_time() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
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
                time: Some(Time::NotSynchronized(Timestamp::new(65536))), // 0xFFFE + 2
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(header(), objects, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn can_calculate_maximum_timestamp() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
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
                time: Some(Time::Synchronized(Timestamp::max())),
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(header(), objects, &mut handler);
        assert!(handler.is_empty());
    }

    #[test]
    fn cto_overflow_of_u48_yields_invalid_time() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
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
                time: None,
            },
            0x07,
        );

        handler.expect(Header::Binary(vec![expected]));
        extract_measurements(header(), objects, &mut handler);
        assert!(handler.is_empty());
    }
}
