// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::gen::count::CountVariation;
use crate::app::measurement::*;
use crate::app::parse::count::CountSequence;
use crate::app::parse::parser::{HeaderCollection, ObjectHeader};
use crate::app::variations::*;
use crate::app::{ObjectParseError, ResponseHeader};
use crate::master::{HeaderInfo, ReadHandler};
use crate::master::{ReadType, TaskError};
use crate::util::skipped::Skipped;

/// Extract measurements from a processable response, sinking them into
/// something that implements `MeasurementHandler`
pub(crate) async fn extract_measurements(
    read_type: ReadType,
    header: ResponseHeader,
    objects: ProcessableResponse<'_>,
    handler: &mut dyn ReadHandler,
) -> Delivered {
    handler.begin_fragment(read_type, header).get().await;
    let delivered = extract_measurements_inner(objects, handler);
    handler.end_fragment(read_type, header).get().await;
    delivered
}

/// Extract measurements from a processable response, sinking them into
/// something that implements `MeasurementHandler`
pub(crate) fn extract_measurements_inner(
    objects: ProcessableResponse<'_>,
    handler: &mut dyn ReadHandler,
) -> Delivered {
    let mut delivered = Delivered::default();
    let mut cto = None;
    let mut ignored = Skipped::new();
    for header in objects.objects.iter() {
        match extract_header(&header, cto, handler) {
            Extracted::Handled => delivered.add(&header),
            Extracted::Cto(x) => cto = x,
            Extracted::Ignored => ignored.add((header.variation(), header.qualifier())),
        }
    }
    if let Some(((variation, qualifier), count)) = ignored.first() {
        tracing::warn!("ignored header variation: {variation} qualifier: {qualifier:?}{count}");
    }
    delivered
}

/// What extraction passed to the handler
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Delivered {
    /// true if the values of any header were passed to the handler
    pub(crate) values: bool,
    /// true if any of those values were events
    pub(crate) events: bool,
}

impl Delivered {
    /// Add a header whose values were passed to the handler
    pub(crate) fn add(&mut self, header: &ObjectHeader) {
        if header.has_objects() {
            self.values = true;
            self.events = self.events || is_event(header);
        }
    }

    /// Combine with what another fragment of the same read delivered
    pub(crate) fn merge(&mut self, other: Self) {
        self.values |= other.values;
        self.events |= other.events;
    }
}

/// True if the header's variation is an event variation
fn is_event(header: &ObjectHeader) -> bool {
    match header {
        ObjectHeader::CountAndPrefix8(_, variation) => variation.get_header_info().is_event,
        ObjectHeader::CountAndPrefix16(_, variation) => variation.get_header_info().is_event,
        ObjectHeader::AllObjects(_)
        | ObjectHeader::Range(..)
        | ObjectHeader::Count(..)
        | ObjectHeader::FreeFormat(_)
        | ObjectHeader::Read(..) => false,
    }
}

/// The objects of a response that the master can process: they parse, and every object with
/// relative time follows a common time of occurrence in the same fragment
#[derive(Copy, Clone)]
pub(crate) struct ProcessableResponse<'a> {
    objects: HeaderCollection<'a>,
}

impl<'a> ProcessableResponse<'a> {
    /// Check that the master can process a response's objects
    pub(crate) fn new(
        objects: Result<HeaderCollection<'a>, ObjectParseError>,
    ) -> Result<Self, InvalidResponse> {
        let objects = objects.map_err(InvalidResponse::Malformed)?;
        match objects.relative_time_without_cto() {
            Some(variation) => Err(InvalidResponse::RelativeTimeWithoutCto(variation)),
            None => Ok(Self { objects }),
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
                HeaderInfo::new(header.variation(), header.qualifier(), false, false),
                item.time,
            );
            true
        } else {
            false
        }
    }

    if let Some(cto) = header.next_cto(cto) {
        return Extracted::Cto(cto);
    }

    let handled = match &header {
        // handle g50v1 - absolute time (only count == 1 is valid)
        ObjectHeader::Count(_, 1, CountVariation::Group50Var1(seq)) => {
            extract_g50v1(seq, header, handler)
        }
        // everything else
        ObjectHeader::Range(_, _, var) => var.extract_measurements_to(header.qualifier(), handler),
        ObjectHeader::CountAndPrefix8(_, var) => var.extract_measurements_to(cto, handler),
        ObjectHeader::CountAndPrefix16(_, var) => var.extract_measurements_to(cto, handler),
        ObjectHeader::AllObjects(_)
        | ObjectHeader::Count(..)
        | ObjectHeader::FreeFormat(_)
        | ObjectHeader::Read(..) => false,
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
    use crate::app::*;
    use crate::master::extract_recorder::{extract, Call, Kind, Recorder, Value};
    use crate::master::test_objects::{RELATIVE_TIME_EVENT, TRUNCATED_ANALOG_EVENT};
    use crate::master::HeaderInfo;

    use super::*;

    /// Each callback and its values, without the information of its header
    fn callbacks(recorder: Recorder) -> Vec<(Kind, Vec<Value>)> {
        recorder
            .calls
            .into_iter()
            .map(|call| (call.kind, call.values))
            .collect()
    }

    /// The callbacks for response objects that the master can process
    fn extract_valid(objects: &[u8]) -> Vec<(Kind, Vec<Value>)> {
        let (result, recorder) = extract(objects);
        result.unwrap();
        callbacks(recorder)
    }

    fn validate(objects: &[u8]) -> Result<(), InvalidResponse> {
        extract(objects).0.map(|_| ())
    }

    #[test]
    fn time_delay_without_cto_is_valid() {
        // g52v1 count: 1 time: 5
        assert_eq!(validate(&[0x34, 0x01, 0x07, 0x01, 0x05, 0x00]), Ok(()));
        // g52v2 count: 1 time: 5
        assert_eq!(validate(&[0x34, 0x02, 0x07, 0x01, 0x05, 0x00]), Ok(()));
    }

    #[test]
    fn malformed_objects_are_invalid() {
        assert_eq!(
            validate(TRUNCATED_ANALOG_EVENT),
            Err(InvalidResponse::Malformed(
                ObjectParseError::InsufficientBytes
            ))
        );
    }

    #[test]
    fn malformed_objects_after_relative_time_without_cto_are_malformed() {
        assert_eq!(
            validate(&[RELATIVE_TIME_EVENT, TRUNCATED_ANALOG_EVENT].concat()),
            Err(InvalidResponse::Malformed(
                ObjectParseError::InsufficientBytes
            ))
        );
    }

    #[test]
    fn handles_frozen_analog_event() {
        let calls = extract_valid(&[
            33, 3, 0x17, 0x01, 0x07, 0x01, 0xDE, 0xAD, 0xCA, 0xFE, 0x01, 0x02, 0x03, 0x04, 0x05,
            0x06,
        ]);

        let expected = Value::FrozenAnalog(
            FrozenAnalogInput {
                value: i32::from_le_bytes([0xDE, 0xAD, 0xCA, 0xFE]).into(),
                flags: Flags::ONLINE,
                time: Some(Time::Synchronized(Timestamp::new(0x060504030201))),
            },
            0x07,
        );

        assert_eq!(calls, [(Kind::FrozenAnalogInput, vec![expected])]);
    }

    #[test]
    fn handles_analog_input_dead_band() {
        let calls = extract_valid(&[0x22, 1, 0x00, 0x01, 0x02, 0xCA, 0xFE, 0xDE, 0xAD]);

        let items = vec![
            Value::AnalogDeadBand(AnalogInputDeadBand::U16(0xFECA), 0x01),
            Value::AnalogDeadBand(AnalogInputDeadBand::U16(0xADDE), 0x02),
        ];

        assert_eq!(calls, [(Kind::AnalogInputDeadBand, items)]);
    }

    #[test]
    fn handles_static_frozen_analog() {
        let calls = extract_valid(&[31, 2, 0x00, 0x07, 0x07, 0x01, 0xCA, 0xFE]);

        let expected = Value::FrozenAnalog(
            FrozenAnalogInput {
                value: i16::from_le_bytes([0xCA, 0xFE]).into(),
                flags: Flags::ONLINE,
                time: None,
            },
            0x07,
        );

        assert_eq!(calls, [(Kind::FrozenAnalogInput, vec![expected])]);
    }

    #[test]
    fn handles_device_attrs() {
        let calls = extract_valid(&[
            // group 0 var 252 - default set == 0 - one byte count and prefix
            0, 252, 0x17, 0x01, 0x00, 0x01, 0x04, b'A', b'C', b'M', b'E',
            // group 0 var 217 - default set == 0 - one byte start/stop
            0, 217, 0x00, 0x00, 0x00, 0x02, 0x02, 0xFE, 0xCA,
            // group 0 var 42 - private set == 7 - one byte count and prefix
            0, 42, 0x17, 0x01, 0x07, 0x01, 0x03, b'F', b'O', b'O',
        ]);

        let h1 = Value::KnownAttribute(StringAttr::DeviceManufacturersName.with_value("ACME"));
        let h2 = Value::KnownAttribute(UIntAttr::LocalTimingAccuracy.with_value(0xCAFE));
        let h3 = Value::Attribute(OwnedAttribute::new(
            AttrSet::Private(7),
            42,
            OwnedAttrValue::VisibleString("FOO".to_string()),
        ));
        assert_eq!(
            calls,
            [
                (Kind::DeviceAttribute, vec![h1]),
                (Kind::DeviceAttribute, vec![h2]),
                (Kind::DeviceAttribute, vec![h3]),
            ]
        );
    }

    #[test]
    fn handles_g13v1_and_g13v2() {
        let calls = extract_valid(&[
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
        ]);

        assert_eq!(
            calls,
            [
                (
                    Kind::BinaryOutputCommandEvent,
                    vec![Value::BinaryCommandEvent(
                        BinaryOutputCommandEvent {
                            commanded_state: true,
                            status: CommandStatus::FormatError,
                            time: None,
                        },
                        7
                    )]
                ),
                (
                    Kind::BinaryOutputCommandEvent,
                    vec![Value::BinaryCommandEvent(
                        BinaryOutputCommandEvent {
                            commanded_state: false,
                            status: CommandStatus::Success,
                            time: Some(Time::Synchronized(Timestamp::new(0xFFEEDDCCBBAA))),
                        },
                        255
                    )]
                )
            ]
        );
    }

    #[test]
    fn handles_g43_object() {
        let calls = extract_valid(&[
            43, // g43v2
            2, 0x17, 0x01, // count == 1
            0x07, // index 7,
            0x08, // status  == Too many ops
            0xFE, 0xCA,
        ]);

        assert_eq!(
            calls,
            [(
                Kind::AnalogOutputCommandEvent,
                vec![Value::AnalogCommandEvent(
                    AnalogOutputCommandEvent {
                        status: CommandStatus::TooManyOps,
                        commanded_value: AnalogCommandValue::I16(0xCAFEu16.cast_signed()),
                        time: None,
                    },
                    7
                )]
            )]
        );
    }

    #[test]
    fn handles_g102v1() {
        let calls = extract_valid(&[
            102, // g43v2
            1, 0x00, // 1 byte start/stop
            0x07, // index 7,
            0x08, // index 8
            0xFE, 0xCA,
        ]);

        assert_eq!(
            calls,
            [(
                Kind::UnsignedInteger,
                vec![
                    Value::UnsignedInteger(UnsignedInteger { value: 0xFE }, 7),
                    Value::UnsignedInteger(UnsignedInteger { value: 0xCA }, 8)
                ]
            )]
        );
    }

    fn binary(value: bool, time: Option<Time>, index: u16) -> Value {
        let flags = if value {
            Flags::new(0x81)
        } else {
            Flags::ONLINE
        };
        Value::Binary(BinaryInput { value, flags, time }, index)
    }

    fn call(info: HeaderInfo, kind: Kind, values: Vec<Value>) -> Call {
        Call { kind, info, values }
    }

    #[test]
    fn extracts_ranges_at_both_widths() {
        let (result, recorder) = extract(&[
            // g1v2, 1-byte start/stop [7, 8]
            0x01, 0x02, 0x00, 0x07, 0x08, 0x81, 0x01, //
            // g1v2, 2-byte start/stop [256, 257]
            0x01, 0x02, 0x01, 0x00, 0x01, 0x01, 0x01, 0x01, 0x81,
        ]);

        assert!(result.unwrap().values);
        assert_eq!(
            recorder.calls,
            [
                call(
                    HeaderInfo::new(Variation::Group1Var2, QualifierCode::Range8, false, true),
                    Kind::BinaryInput,
                    vec![binary(true, None, 7), binary(false, None, 8)]
                ),
                call(
                    HeaderInfo::new(Variation::Group1Var2, QualifierCode::Range16, false, true),
                    Kind::BinaryInput,
                    vec![binary(false, None, 256), binary(true, None, 257)]
                ),
            ]
        );
    }

    #[test]
    fn extracts_count_and_prefix_at_both_widths() {
        let (result, recorder) = extract(&[
            // g2v1, 1-byte count and prefix, index 5
            0x02, 0x01, 0x17, 0x01, 0x05, 0x81, //
            // g2v1, 2-byte count and prefix, index 261
            0x02, 0x01, 0x28, 0x01, 0x00, 0x05, 0x01, 0x01,
        ]);

        assert!(result.unwrap().values);
        assert_eq!(
            recorder.calls,
            [
                call(
                    HeaderInfo::new(
                        Variation::Group2Var1,
                        QualifierCode::CountAndPrefix8,
                        true,
                        true
                    ),
                    Kind::BinaryInput,
                    vec![binary(true, None, 5)]
                ),
                call(
                    HeaderInfo::new(
                        Variation::Group2Var1,
                        QualifierCode::CountAndPrefix16,
                        true,
                        true
                    ),
                    Kind::BinaryInput,
                    vec![binary(false, None, 261)]
                ),
            ]
        );
    }

    #[test]
    fn empty_count_and_prefix_headers_deliver_nothing_at_both_widths() {
        for input in [
            [0x02, 0x01, 0x17, 0x00].as_slice(),
            [0x02, 0x01, 0x28, 0x00, 0x00].as_slice(),
        ] {
            let (result, recorder) = extract(input);
            assert!(!result.unwrap().values);
            assert_eq!(callbacks(recorder), [(Kind::BinaryInput, vec![])]);
        }
    }
}
