use crate::app::gen::count::CountVariation;
use crate::app::measurement::*;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, HeaderDetails, ObjectHeader};
use crate::app::variations::*;
use crate::app::{ObjectParseError, ResponseHeader};
use crate::master::{HeaderInfo, ReadHandler};
use crate::master::{ReadType, TaskError};

/// Extract measurements from a HeaderCollection, sinking them into
/// something that implements `MeasurementHandler`
///
/// Returns true if the values of any header were passed to the handler
pub(crate) async fn extract_measurements(
    read_type: ReadType,
    header: ResponseHeader,
    objects: HeaderCollection<'_>,
    handler: &mut dyn ReadHandler,
) -> bool {
    handler.begin_fragment(read_type, header).get().await;
    let delivered = extract_measurements_inner(objects, handler);
    handler.end_fragment(read_type, header).get().await;
    delivered
}

/// Extract measurements from a HeaderCollection, sinking them into
/// something that implements `MeasurementHandler`
pub(crate) fn extract_measurements_inner(
    objects: HeaderCollection<'_>,
    handler: &mut dyn ReadHandler,
) -> bool {
    let mut delivered = false;
    let mut cto = None;
    for header in objects.iter() {
        match extract_header(&header, cto, handler) {
            Extracted::Handled => delivered |= has_objects(&header.details),
            Extracted::Cto(x) => cto = x,
            Extracted::Ignored => {
                tracing::warn!(
                    "ignored header variation: {} qualifier: {:?}",
                    &header.variation,
                    &header.details.qualifier()
                );
            }
        }
    }
    delivered
}

/// True if the header carries at least one object
fn has_objects(details: &HeaderDetails) -> bool {
    match details {
        HeaderDetails::AllObjects(_) => false,
        // a range covers at least one index
        HeaderDetails::OneByteStartStop(..) | HeaderDetails::TwoByteStartStop(..) => true,
        HeaderDetails::OneByteCount(count, _)
        | HeaderDetails::OneByteCountAndPrefix(count, _)
        | HeaderDetails::TwoByteFreeFormat(count, _) => *count > 0,
        HeaderDetails::TwoByteCount(count, _) | HeaderDetails::TwoByteCountAndPrefix(count, _) => {
            *count > 0
        }
    }
}

/// Why the master discards a response without processing or confirming it
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum InvalidResponse {
    /// the objects failed to parse
    Malformed(ObjectParseError),
    /// an object with relative time has no common time of occurrence before it in the fragment
    RelativeTimeWithoutCto(Variation),
}

impl std::fmt::Display for InvalidResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed(err) => write!(f, "malformed objects: {err}"),
            Self::RelativeTimeWithoutCto(variation) => {
                write!(
                    f,
                    "{variation} without a preceding common time of occurrence"
                )
            }
        }
    }
}

impl From<InvalidResponse> for TaskError {
    fn from(err: InvalidResponse) -> Self {
        match err {
            InvalidResponse::Malformed(err) => TaskError::MalformedResponse(err),
            InvalidResponse::RelativeTimeWithoutCto(_) => TaskError::UnexpectedResponseHeaders,
        }
    }
}

/// Check that the master can process a response's objects: they parse, and every object with
/// relative time follows a common time of occurrence in the same fragment
pub(crate) fn validate_response(
    objects: Result<HeaderCollection<'_>, ObjectParseError>,
) -> Result<HeaderCollection<'_>, InvalidResponse> {
    let objects = objects.map_err(InvalidResponse::Malformed)?;
    let mut cto = None;
    for header in objects.iter() {
        if let Some(next) = next_cto(&header.details, cto) {
            cto = next;
        } else if cto.is_none() && has_relative_time(&header) {
            return Err(InvalidResponse::RelativeTimeWithoutCto(header.variation));
        }
    }
    Ok(objects)
}

/// True if the header carries objects whose time is relative to a common time of occurrence
fn has_relative_time(header: &ObjectHeader) -> bool {
    matches!(
        header.variation,
        Variation::Group2Var3 | Variation::Group4Var3
    ) && has_objects(&header.details)
}

/// If the header is a common time of occurrence, the one for the headers that follow it
fn next_cto(details: &HeaderDetails, prev: Option<Time>) -> Option<Option<Time>> {
    let time = match details {
        HeaderDetails::OneByteCount(1, CountVariation::Group51Var1(seq))
        | HeaderDetails::TwoByteCount(1, CountVariation::Group51Var1(seq)) => {
            seq.single().map(|x| Time::Synchronized(x.time))
        }
        HeaderDetails::OneByteCount(1, CountVariation::Group51Var2(seq))
        | HeaderDetails::TwoByteCount(1, CountVariation::Group51Var2(seq)) => {
            seq.single().map(|x| Time::Unsynchronized(x.time))
        }
        _ => return None,
    };
    Some(time.or(prev))
}

/// What [`extract_header`] did with a header
pub(crate) enum Extracted {
    /// the header's values were passed to the handler
    Handled,
    /// the header is a common time of occurrence for the headers that follow
    Cto(Option<Time>),
    /// the header isn't one that the master extracts
    Ignored,
}

/// Pass the values of one header to the handler, given the common time of occurrence
/// of the headers before it
pub(crate) fn extract_header<H: ReadHandler + ?Sized>(
    header: &ObjectHeader,
    cto: Option<Time>,
    handler: &mut H,
) -> Extracted {
    fn extract_g50v1<H: ReadHandler + ?Sized>(
        seq: &CountSequence<Group50Var1>,
        header: &ObjectHeader,
        handler: &mut H,
    ) -> bool {
        if let Some(item) = seq.single() {
            handler.handle_abs_time(
                HeaderInfo::new(header.variation, header.details.qualifier(), false, false),
                item.time,
            );
            true
        } else {
            false
        }
    }

    if let Some(cto) = next_cto(&header.details, cto) {
        return Extracted::Cto(cto);
    }

    let handled = match &header.details {
        // handle g50v1 - absolute time (only count == 1 is valid)
        HeaderDetails::OneByteCount(1, CountVariation::Group50Var1(seq)) => {
            extract_g50v1(seq, header, handler)
        }
        HeaderDetails::TwoByteCount(1, CountVariation::Group50Var1(seq)) => {
            extract_g50v1(seq, header, handler)
        }
        // everything else
        HeaderDetails::OneByteStartStop(_, _, var) => {
            var.extract_measurements_to(header.variation, header.details.qualifier(), handler)
        }
        HeaderDetails::TwoByteStartStop(_, _, var) => {
            var.extract_measurements_to(header.variation, header.details.qualifier(), handler)
        }
        HeaderDetails::OneByteCountAndPrefix(_, var) => var.extract_measurements_to(cto, handler),
        HeaderDetails::TwoByteCountAndPrefix(_, var) => var.extract_measurements_to(cto, handler),
        HeaderDetails::AllObjects(_)
        | HeaderDetails::OneByteCount(..)
        | HeaderDetails::TwoByteCount(..)
        | HeaderDetails::TwoByteFreeFormat(..) => false,
    };

    if handled {
        Extracted::Handled
    } else {
        Extracted::Ignored
    }
}

#[cfg(test)]
mod test {
    use crate::app::attr::*;
    use crate::app::control::CommandStatus;
    use crate::app::parse::options::ParseOptions;
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
        AbsTime(Timestamp),
    }

    #[derive(Default)]
    struct MockHandler {
        received: Vec<Header>,
        infos: Vec<HeaderInfo>,
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
            info: HeaderInfo,
            x: &mut dyn Iterator<Item = (BinaryInput, u16)>,
        ) {
            self.infos.push(info);
            self.received.push(Header::Binary(x.collect()));
        }

        fn handle_double_bit_binary_input(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
        ) {
            panic!("not used by these tests")
        }

        fn handle_binary_output_status(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
        ) {
            panic!("not used by these tests")
        }

        fn handle_counter(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (Counter, u16)>,
        ) {
            panic!("not used by these tests")
        }

        fn handle_frozen_counter(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
        ) {
            panic!("not used by these tests")
        }

        fn handle_analog_input(
            &mut self,
            _info: HeaderInfo,
            _x: &mut dyn Iterator<Item = (AnalogInput, u16)>,
        ) {
            panic!("not used by these tests")
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
            panic!("not used by these tests")
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
            panic!("not used by these tests")
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

        fn handle_abs_time(&mut self, info: HeaderInfo, time: Timestamp) {
            self.infos.push(info);
            self.received.push(Header::AbsTime(time));
        }
    }

    fn validate(objects: &[u8]) -> Result<(), InvalidResponse> {
        validate_response(HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            objects,
        ))
        .map(|_| ())
    }

    #[test]
    fn relative_time_without_cto_is_invalid() {
        // g2v3 count: 1 index: 7 flags: 0x01 time: 0xFFFF
        assert_eq!(
            validate(&[0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF]),
            Err(InvalidResponse::RelativeTimeWithoutCto(
                Variation::Group2Var3
            ))
        );
        // g4v3 count: 1 index: 7 flags: 0x01 time: 0xFFFF
        assert_eq!(
            validate(&[0x04, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF]),
            Err(InvalidResponse::RelativeTimeWithoutCto(
                Variation::Group4Var3
            ))
        );
    }

    #[test]
    fn relative_time_before_cto_is_invalid() {
        assert_eq!(
            validate(&[
                0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF, 0x33, 0x01, 0x07, 0x01, 0x02, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ]),
            Err(InvalidResponse::RelativeTimeWithoutCto(
                Variation::Group2Var3
            ))
        );
    }

    #[test]
    fn relative_time_after_cto_is_valid() {
        // g51v2 time: 2, then g4v3 and g2v3
        assert_eq!(
            validate(&[
                0x33, 0x02, 0x07, 0x01, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x03, 0x17, 0x01,
                0x07, 0x01, 0xFF, 0xFF, 0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF,
            ]),
            Ok(())
        );
    }

    #[test]
    fn relative_time_header_without_objects_is_valid() {
        // g2v3 count: 0
        assert_eq!(validate(&[0x02, 0x03, 0x17, 0x00]), Ok(()));
    }

    #[test]
    fn malformed_objects_are_invalid() {
        // g32v2 count: 1 index: 0, missing its flags and value
        assert_eq!(
            validate(&[0x20, 0x02, 0x17, 0x01, 0x00]),
            Err(InvalidResponse::Malformed(
                ObjectParseError::InsufficientBytes
            ))
        );
    }

    #[test]
    fn g2v3_with_synchronized_cto_yields_synchronized_time() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            ParseOptions::default(),
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
            ParseOptions::default(),
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
            ParseOptions::default(),
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
            ParseOptions::default(),
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
            ParseOptions::default(),
            FunctionCode::Response,
            &[
                33, 3, 0x17, 0x01, 0x07, 0x01, 0xDE, 0xAD, 0xCA, 0xFE, 0x01, 0x02, 0x03, 0x04,
                0x05, 0x06,
            ],
        )
        .unwrap();

        let expected: (FrozenAnalogInput, u16) = (
            FrozenAnalogInput {
                value: i32::from_le_bytes([0xDE, 0xAD, 0xCA, 0xFE]).into(),
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
            ParseOptions::default(),
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
            ParseOptions::default(),
            FunctionCode::Response,
            &[31, 2, 0x00, 0x07, 0x07, 0x01, 0xCA, 0xFE],
        )
        .unwrap();

        let expected: (FrozenAnalogInput, u16) = (
            FrozenAnalogInput {
                value: i16::from_le_bytes([0xCA, 0xFE]).into(),
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
            ParseOptions::default(),
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
            ParseOptions::default(),
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
            ParseOptions::default(),
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
                    commanded_value: AnalogCommandValue::I16(0xCAFEu16.cast_signed()),
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
            ParseOptions::default(),
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

    #[test]
    fn handles_g50v1_with_count_1() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            // g50v1       count: 1   -------- time: 0x060504030201 ------------------
            &[0x32, 0x01, 0x07, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        assert_eq!(
            &handler.pop(),
            &[Header::AbsTime(Timestamp::new(0x060504030201))]
        );
    }

    #[test]
    fn handles_g50v1_with_two_byte_count() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            // g50v1       count: 1 (two bytes)   -------- time: 0xAABBCCDDEEFF ------------------
            &[
                0x32, 0x01, 0x08, 0x01, 0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0xBB, 0xAA,
            ],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        assert_eq!(
            &handler.pop(),
            &[Header::AbsTime(Timestamp::new(0xAABBCCDDEEFF))]
        );
    }

    #[test]
    fn ignores_g50v1_with_count_greater_than_1() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            // g50v1       count: 2   -------- time1 ------ time2 ------
            &[
                0x32, 0x01, 0x07, 0x02, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x11, 0x12, 0x13, 0x14,
                0x15, 0x16,
            ],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        // Should not receive any time values since count > 1
        assert_eq!(&handler.pop(), &[]);
    }

    #[test]
    fn handles_g50v1_with_count_0() {
        let mut handler = MockHandler::new();
        let objects = HeaderCollection::parse(
            ParseOptions::default(),
            FunctionCode::Response,
            // g50v1       count: 0
            &[0x32, 0x01, 0x07, 0x00],
        )
        .unwrap();

        extract_measurements_inner(objects, &mut handler);

        // Should not receive any time values since count == 0
        assert_eq!(&handler.pop(), &[]);
    }

    fn extract(input: &[u8]) -> (bool, MockHandler) {
        let mut handler = MockHandler::new();
        let objects =
            HeaderCollection::parse(ParseOptions::default(), FunctionCode::Response, input)
                .unwrap();
        let delivered = extract_measurements_inner(objects, &mut handler);
        (delivered, handler)
    }

    fn binary(value: bool, time: Option<Time>, index: u16) -> (BinaryInput, u16) {
        let flags = if value {
            Flags::new(0x81)
        } else {
            Flags::ONLINE
        };
        (BinaryInput { value, flags, time }, index)
    }

    #[test]
    fn extracts_ranges_at_both_widths() {
        let (delivered, mut handler) = extract(&[
            // g1v2, 1-byte start/stop [7, 8]
            0x01, 0x02, 0x00, 0x07, 0x08, 0x81, 0x01, //
            // g1v2, 2-byte start/stop [256, 257]
            0x01, 0x02, 0x01, 0x00, 0x01, 0x01, 0x01, 0x01, 0x81,
        ]);

        assert!(delivered);
        assert_eq!(
            handler.infos,
            [
                HeaderInfo::new(Variation::Group1Var2, QualifierCode::Range8, false, true),
                HeaderInfo::new(Variation::Group1Var2, QualifierCode::Range16, false, true),
            ]
        );
        assert_eq!(
            handler.pop(),
            [
                Header::Binary(vec![binary(true, None, 7), binary(false, None, 8)]),
                Header::Binary(vec![binary(false, None, 256), binary(true, None, 257)]),
            ]
        );
    }

    #[test]
    fn extracts_count_and_prefix_at_both_widths() {
        let (delivered, mut handler) = extract(&[
            // g2v1, 1-byte count and prefix, index 5
            0x02, 0x01, 0x17, 0x01, 0x05, 0x81, //
            // g2v1, 2-byte count and prefix, index 261
            0x02, 0x01, 0x28, 0x01, 0x00, 0x05, 0x01, 0x01,
        ]);

        assert!(delivered);
        assert_eq!(
            handler.infos,
            [
                HeaderInfo::new(
                    Variation::Group2Var1,
                    QualifierCode::CountAndPrefix8,
                    true,
                    true
                ),
                HeaderInfo::new(
                    Variation::Group2Var1,
                    QualifierCode::CountAndPrefix16,
                    true,
                    true
                ),
            ]
        );
        assert_eq!(
            handler.pop(),
            [
                Header::Binary(vec![binary(true, None, 5)]),
                Header::Binary(vec![binary(false, None, 261)]),
            ]
        );
    }

    #[test]
    fn empty_count_and_prefix_headers_deliver_nothing_at_both_widths() {
        for input in [
            [0x02, 0x01, 0x17, 0x00].as_slice(),
            [0x02, 0x01, 0x28, 0x00, 0x00].as_slice(),
        ] {
            let (delivered, mut handler) = extract(input);
            assert!(!delivered);
            assert_eq!(handler.pop(), [Header::Binary(vec![])]);
        }
    }

    #[test]
    fn abs_time_is_extracted_at_both_widths() {
        let (delivered, mut handler) = extract(&[
            // g50v1, 1-byte count of 1
            0x32, 0x01, 0x07, 0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, //
            // g50v1, 2-byte count of 1
            0x32, 0x01, 0x08, 0x01, 0x00, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
        ]);

        assert!(delivered);
        assert_eq!(
            handler.infos,
            [
                HeaderInfo::new(Variation::Group50Var1, QualifierCode::Count8, false, false),
                HeaderInfo::new(Variation::Group50Var1, QualifierCode::Count16, false, false),
            ]
        );
        assert_eq!(
            handler.pop(),
            [
                Header::AbsTime(Timestamp::new(0x060504030201)),
                Header::AbsTime(Timestamp::new(0x161514131211)),
            ]
        );
    }

    #[test]
    fn abs_time_with_a_two_byte_count_other_than_one_is_ignored() {
        for input in [
            [0x32, 0x01, 0x08, 0x00, 0x00].as_slice(),
            [
                0x32, 0x01, 0x08, 0x02, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x11, 0x12, 0x13,
                0x14, 0x15, 0x16,
            ]
            .as_slice(),
        ] {
            let (delivered, mut handler) = extract(input);
            assert!(!delivered);
            assert_eq!(handler.pop(), []);
        }
    }

    // g2v3, 1-byte count and prefix, index 7, relative time 0xFFFF
    const G2V3_INDEX_7: [u8; 8] = [0x02, 0x03, 0x17, 0x01, 0x07, 0x01, 0xFF, 0xFF];

    #[test]
    fn cto_with_a_two_byte_count_applies_to_the_next_header() {
        for (variation, time) in [
            (1, Time::Synchronized(Timestamp::new(0x1_0001))),
            (2, Time::Unsynchronized(Timestamp::new(0x1_0001))),
        ] {
            // g51 var 1 or 2, 2-byte count of 1, time 2
            let mut input = vec![
                0x33, variation, 0x08, 0x01, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00,
            ];
            input.extend(G2V3_INDEX_7);

            let (delivered, mut handler) = extract(&input);

            assert!(delivered);
            assert_eq!(
                handler.pop(),
                [Header::Binary(vec![binary(false, Some(time), 7)])]
            );
        }
    }

    #[test]
    fn cto_with_a_count_other_than_one_is_ignored_at_both_widths() {
        for cto in [
            // g51v1, 1-byte count of 0
            vec![0x33, 0x01, 0x07, 0x00],
            // g51v1, 2-byte count of 0
            vec![0x33, 0x01, 0x08, 0x00, 0x00],
            // g51v1, 1-byte count of 2
            vec![
                0x33, 0x01, 0x07, 0x02, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            // g51v2, 2-byte count of 2
            vec![
                0x33, 0x02, 0x08, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00,
                0x00, 0x00, 0x00,
            ],
        ] {
            let mut input = cto;
            input.extend(G2V3_INDEX_7);

            let (delivered, mut handler) = extract(&input);

            assert!(delivered);
            assert_eq!(
                handler.pop(),
                [Header::Binary(vec![binary(false, None, 7)])]
            );
        }
    }
}
