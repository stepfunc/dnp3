use crate::app::gen::count::CountVariation;
use crate::app::measurement::*;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};
use crate::app::variations::*;
use crate::app::ResponseHeader;
use crate::master::handler::ReadHandler;
use crate::master::ReadType;

/// Extract measurements from a HeaderCollection, sinking them into
/// something that implements `MeasurementHandler`
pub(crate) async fn extract_measurements(
    read_type: ReadType,
    header: ResponseHeader,
    objects: HeaderCollection<'_>,
    handler: &mut dyn ReadHandler,
) {
    handler.begin_fragment(read_type, header).get().await;
    extract_measurements_inner(objects, handler);
    handler.end_fragment(read_type, header).get().await;
}

/// Extract measurements from a HeaderCollection, sinking them into
/// something that implements `MeasurementHandler`
pub(crate) fn extract_measurements_inner(
    objects: HeaderCollection<'_>,
    handler: &mut dyn ReadHandler,
) {
    fn extract_cto_g51v1(prev: Option<Time>, item: Option<Group51Var1>) -> Option<Time> {
        item.map_or(prev, |x| Some(Time::Synchronized(x.time)))
    }

    fn extract_cto_g51v2(prev: Option<Time>, item: Option<Group51Var2>) -> Option<Time> {
        item.map_or(prev, |x| Some(Time::Unsynchronized(x.time)))
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
                "ignored header variation: {} qualifier: {:?}",
                &header.variation,
                &header.details.qualifier()
            );
        }

        // if we didn't return early b/c the header is a CTO, then we just return the previous value
        cto
    }

    objects
        .iter()
        .fold(None, |cto, header| handle(cto, header, handler));
}

#[cfg(test)]
mod test {
    use crate::app::parse::parser::HeaderCollection;
    use crate::app::*;
    use crate::master::handler::{HeaderInfo, ReadHandler};

    use super::*;

    fn header() -> ResponseHeader {
        ResponseHeader::new(
            ControlField::from(0xC0),
            ResponseFunction::Response,
            Iin::default(),
        )
    }

    #[derive(Debug, PartialEq)]
    enum Header {
        Binary(Vec<(BinaryInput, u16)>),
        FrozenAnalog(Vec<(FrozenAnalogInput, u16)>),
        AnalogDeadBand(Vec<(AnalogInputDeadBand, u16)>),
    }

    #[derive(Default)]
    struct MockHandler {
        received: Vec<Header>,
    }

    impl MockHandler {
        fn new() -> Self {
            Default::default()
        }

        fn pop(&mut self) -> Vec<Header> {
            let mut received = Default::default();
            std::mem::swap(&mut received, &mut self.received);
            received
        }
    }

    impl ReadHandler for MockHandler {
        fn begin_fragment(
            &mut self,
            _read_type: ReadType,
            _header: ResponseHeader,
        ) -> MaybeAsync<()> {
            MaybeAsync::ready(())
        }
        fn end_fragment(
            &mut self,
            _read_type: ReadType,
            _header: ResponseHeader,
        ) -> MaybeAsync<()> {
            MaybeAsync::ready(())
        }

        fn handle_binary_input(
            &mut self,
            _info: HeaderInfo,
            x: &mut dyn Iterator<Item = (BinaryInput, u16)>,
        ) {
            self.received.push(Header::Binary(x.collect()));
        }

        fn handle_double_bit_binary_input(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
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

        fn handle_analog_input(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (AnalogInput, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_analog_input_dead_band(
            &mut self,
            _info: HeaderInfo,
            iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
        ) {
            self.received.push(Header::AnalogDeadBand(iter.collect()))
        }

        fn handle_frozen_analog_input(
            &mut self,
            _info: HeaderInfo,
            x: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
        ) {
            self.received.push(Header::FrozenAnalog(x.collect()))
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
            _x: &mut dyn Iterator<Item = (&'a [u8], u16)>,
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

        let expected: (BinaryInput, u16) = (
            BinaryInput {
                value: false,
                flags: Flags::ONLINE,
                time: None,
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);

        assert_eq!(handler.pop(), &[Header::Binary(vec![expected])]);
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

        let expected = (
            BinaryInput {
                value: false,
                flags: Flags::ONLINE,
                time: Some(Time::Synchronized(Timestamp::new(65537))), // 0xFFFF + 2
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::Binary(vec![expected])]);
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

        let expected: (BinaryInput, u16) = (
            BinaryInput {
                value: false,
                flags: Flags::ONLINE,
                time: Some(Time::unsynchronized(65536)), // 0xFFFE + 2
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::Binary(vec![expected])]);
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

        let expected: (BinaryInput, u16) = (
            BinaryInput {
                value: false,
                flags: Flags::ONLINE,
                time: Some(Time::Synchronized(Timestamp::max())),
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::Binary(vec![expected])]);
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

        let expected: (BinaryInput, u16) = (
            BinaryInput {
                value: false,
                flags: Flags::ONLINE,
                time: None,
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::Binary(vec![expected])]);
    }

    #[test]
    fn handles_frozen_analog_event() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::Response,
            &[
                33, 3, 0x17, 0x01, 0x07, 0x01, 0xDE, 0xAD, 0xCA, 0xFE, 0x01, 0x02, 0x03, 0x04,
                0x05, 0x06,
            ],
        )
        .unwrap();

        let expected: (FrozenAnalogInput, u16) = (
            FrozenAnalogInput {
                value: i32::from_le_bytes([0xDE, 0xAD, 0xCA, 0xFE]) as f64,
                flags: Flags::ONLINE,
                time: Some(Time::Synchronized(Timestamp::new(0x060504030201))),
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::FrozenAnalog(vec![expected])]);
    }

    #[test]
    fn handles_analog_input_dead_band() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::Response,
            &[0x22, 1, 0x00, 0x01, 0x02, 0xCA, 0xFE, 0xDE, 0xAD],
        )
        .unwrap();

        let items = vec![
            (AnalogInputDeadBand::U16(0xFECA), 0x01),
            (AnalogInputDeadBand::U16(0xADDE), 0x02),
        ];

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::AnalogDeadBand(items)]);
    }

    #[test]
    fn handles_static_frozen_analog() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::Response,
            &[31, 2, 0x00, 0x07, 0x07, 0x01, 0xCA, 0xFE],
        )
        .unwrap();

        let expected: (FrozenAnalogInput, u16) = (
            FrozenAnalogInput {
                value: i16::from_le_bytes([0xCA, 0xFE]) as f64,
                flags: Flags::ONLINE,
                time: None,
            },
            0x07,
        );

        extract_measurements_inner(objects, &mut handler);
        assert_eq!(&handler.pop(), &[Header::FrozenAnalog(vec![expected])]);
    }
}
