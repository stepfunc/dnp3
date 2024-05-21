use crate::app::gen::count::CountVariation;
use crate::app::measurement::*;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};
use crate::app::variations::*;
use crate::app::ResponseHeader;
use crate::master::ReadHandler;
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
                var.extract_measurements_to(header.variation, header.details.qualifier(), handler)
            }
            HeaderDetails::TwoByteStartStop(_, _, var) => {
                var.extract_measurements_to(header.variation, header.details.qualifier(), handler)
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
    use crate::app::attr::*;
    use crate::app::control::CommandStatus;
    use crate::app::parse::parser::HeaderCollection;
    use crate::app::*;
    use crate::master::{HeaderInfo, ReadHandler};

    use super::*;

    fn header() -> ResponseHeader {
        ResponseHeader::new(
            ControlField::from(0xC0),
            ResponseFunction::Response,
            Iin::default(),
        )
    }

    #[derive(Clone, Debug, PartialEq)]
    enum Known {
        String(StringAttr, String),
        UInt(UIntAttr, u32),
        Bool(BoolAttr, bool),
        List(VariationListAttr, Vec<AttrItem>),
        Float(FloatAttr, FloatType),
        Octets(OctetStringAttr, Vec<u8>),
        Time(TimeAttr, Timestamp),
    }

    #[derive(Debug, PartialEq)]
    enum Header {
        Binary(Vec<(BinaryInput, u16)>),
        FrozenAnalog(Vec<(FrozenAnalogInput, u16)>),
        AnalogDeadBand(Vec<(AnalogInputDeadBand, u16)>),
        KnownAttr(Known),
        UnknownAttr(AttrSet, u8, String),
        BinaryCommandEvent(Vec<(BinaryOutputCommandEvent, u16)>),
        AnalogCommandEvent(Vec<(AnalogOutputCommandEvent, u16)>),
        G102(Vec<(UnsignedInteger, u16)>),
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

        fn handle_frozen_analog_input(
            &mut self,
            _info: HeaderInfo,
            x: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
        ) {
            self.received.push(Header::FrozenAnalog(x.collect()))
        }

        fn handle_analog_input_dead_band(
            &mut self,
            _info: HeaderInfo,
            iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
        ) {
            self.received.push(Header::AnalogDeadBand(iter.collect()))
        }

        fn handle_analog_output_status(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
        ) {
            unimplemented!()
        }

        fn handle_binary_output_command_event(
            &mut self,
            _info: HeaderInfo,
            x: &mut dyn Iterator<Item = (BinaryOutputCommandEvent, u16)>,
        ) {
            self.received.push(Header::BinaryCommandEvent(x.collect()))
        }

        fn handle_analog_output_command_event(
            &mut self,
            _info: HeaderInfo,
            x: &mut dyn Iterator<Item = (AnalogOutputCommandEvent, u16)>,
        ) {
            self.received.push(Header::AnalogCommandEvent(x.collect()))
        }

        fn handle_unsigned_integer(
            &mut self,
            _info: HeaderInfo,
            x: &mut dyn Iterator<Item = (UnsignedInteger, u16)>,
        ) {
            self.received.push(Header::G102(x.collect()))
        }

        fn handle_octet_string(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (&[u8], u16)>,
        ) {
            unimplemented!()
        }

        fn handle_device_attribute(&mut self, _info: HeaderInfo, attr: AnyAttribute) {
            match attr {
                AnyAttribute::Other(x) => {
                    let value = x.value.expect_vstr().unwrap();
                    self.received
                        .push(Header::UnknownAttr(x.set, x.variation, value.to_string()));
                }
                AnyAttribute::Known(x) => {
                    let known = match x {
                        KnownAttribute::AttributeList(x, items) => {
                            Known::List(x, items.iter().collect())
                        }
                        KnownAttribute::String(x, v) => Known::String(x, v.to_string()),
                        KnownAttribute::UInt(x, v) => Known::UInt(x, v),
                        KnownAttribute::Bool(x, v) => Known::Bool(x, v),
                        KnownAttribute::Float(x, v) => Known::Float(x, v),
                        KnownAttribute::OctetString(x, v) => Known::Octets(x, v.to_vec()),
                        KnownAttribute::DNP3Time(x, v) => Known::Time(x, v),
                    };
                    self.received.push(Header::KnownAttr(known));
                }
            }
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

    #[test]
    fn handles_device_attrs() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::Response,
            &[
                // group 0 var 252 - default set == 0 - one byte count and prefix
                0, 252, 0x17, 0x01, 0x00, 0x01, 0x04, b'A', b'C', b'M', b'E',
                // group 0 var 217 - default set == 0 - one byte start/stop
                0, 217, 0x00, 0x00, 0x00, 0x02, 0x02, 0xFE, 0xCA,
                // group 0 var 42 - private set == 7 - one byte count and prefix
                0, 42, 0x17, 0x01, 0x07, 0x01, 0x03, b'F', b'O', b'O',
            ],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        let h1 = Header::KnownAttr(Known::String(
            StringAttr::DeviceManufacturersName,
            "ACME".to_string(),
        ));
        let h2 = Header::KnownAttr(Known::UInt(UIntAttr::LocalTimingAccuracy, 0xCAFE));
        let h3 = Header::UnknownAttr(AttrSet::Private(7), 42, "FOO".to_string());
        assert_eq!(&handler.pop(), &[h1, h2, h3]);
    }

    #[test]
    fn handles_g13v1_and_g13v2() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::UnsolicitedResponse,
            &[
                13, // g13v1
                1,
                0x17,
                0x01,        // count == 1
                0x07,        // index 7,
                0b1000_0011, // command_state = true, status == FORMAT_ERROR
                13,          // g13v2
                2,
                0x17,
                0x01,        // count == 1
                0xFF,        // index 255
                0b0000_0000, // command_state = false, status == SUCCESS
                // timestamp 48
                0xAA,
                0xBB,
                0xCC,
                0xDD,
                0xEE,
                0xFF,
            ],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        assert_eq!(
            &handler.pop(),
            &[
                Header::BinaryCommandEvent(vec![(
                    BinaryOutputCommandEvent {
                        commanded_state: true,
                        status: CommandStatus::FormatError,
                        time: None,
                    },
                    7
                )]),
                Header::BinaryCommandEvent(vec![(
                    BinaryOutputCommandEvent {
                        commanded_state: false,
                        status: CommandStatus::Success,
                        time: Some(Time::Synchronized(Timestamp::new(0xFFEEDDCCBBAA))),
                    },
                    255
                )])
            ]
        );
    }

    #[test]
    fn handles_g43_object() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::UnsolicitedResponse,
            &[
                43, // g43v2
                2, 0x17, 0x01, // count == 1
                0x07, // index 7,
                0x08, // status  == Too many ops
                0xFE, 0xCA,
            ],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        assert_eq!(
            &handler.pop(),
            &[Header::AnalogCommandEvent(vec![(
                AnalogOutputCommandEvent {
                    status: CommandStatus::TooManyOps,
                    commanded_value: AnalogCommandValue::I16(0xCAFEu16 as i16),
                    time: None,
                },
                7
            )])]
        );
    }

    #[test]
    fn handles_g102v1() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            FunctionCode::UnsolicitedResponse,
            &[
                102, // g43v2
                1, 0x00, // 1 byte start/stop
                0x07, // index 7,
                0x08, // index 8
                0xFE, 0xCA,
            ],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        assert_eq!(
            &handler.pop(),
            &[Header::G102(vec![
                (UnsignedInteger { value: 0xFE }, 7),
                (UnsignedInteger { value: 0xCA }, 8)
            ])]
        );
    }
}
