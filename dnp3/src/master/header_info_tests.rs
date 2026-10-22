//! Pins the header information that master extraction passes to a `ReadHandler` for every
//! response header: the callback, and the variation, qualifier, event and flags of `HeaderInfo`

use crate::app::attr::{AnyAttribute, AttrSet};
use crate::app::measurement as m;
use crate::app::parse::header_identity_tests::accepted_headers;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::HeaderCollection;
use crate::app::{FunctionCode, QualifierCode, Timestamp, Variation};
use crate::master::extract::extract_measurements_inner;
use crate::master::{HeaderInfo, ReadHandler};

use QualifierCode as Q;

/// The callbacks of a `ReadHandler` that carry values
#[derive(Copy, Clone, Debug, PartialEq)]
enum Kind {
    BinaryInput,
    DoubleBitBinaryInput,
    BinaryOutputStatus,
    Counter,
    FrozenCounter,
    AnalogInput,
    FrozenAnalogInput,
    AnalogInputDeadBand,
    AnalogOutputStatus,
    AnalogOutputCommandEvent,
    BinaryOutputCommandEvent,
    UnsignedInteger,
    OctetString,
    DeviceAttribute,
    AbsTime,
}

/// One callback: its kind, header information and the indices of its values
#[derive(Clone, Debug, PartialEq)]
struct Call {
    kind: Kind,
    info: HeaderInfo,
    indices: Vec<u16>,
}

#[derive(Default)]
struct Recorder {
    calls: Vec<Call>,
}

impl Recorder {
    fn push<T>(&mut self, kind: Kind, info: HeaderInfo, iter: &mut dyn Iterator<Item = (T, u16)>) {
        let indices = iter.map(|(_, index)| index).collect();
        self.calls.push(Call {
            kind,
            info,
            indices,
        });
    }
}

impl ReadHandler for Recorder {
    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::BinaryInput, u16)>,
    ) {
        self.push(Kind::BinaryInput, info, iter)
    }

    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::DoubleBitBinaryInput, u16)>,
    ) {
        self.push(Kind::DoubleBitBinaryInput, info, iter)
    }

    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::BinaryOutputStatus, u16)>,
    ) {
        self.push(Kind::BinaryOutputStatus, info, iter)
    }

    fn handle_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::Counter, u16)>,
    ) {
        self.push(Kind::Counter, info, iter)
    }

    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::FrozenCounter, u16)>,
    ) {
        self.push(Kind::FrozenCounter, info, iter)
    }

    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::AnalogInput, u16)>,
    ) {
        self.push(Kind::AnalogInput, info, iter)
    }

    fn handle_frozen_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::FrozenAnalogInput, u16)>,
    ) {
        self.push(Kind::FrozenAnalogInput, info, iter)
    }

    fn handle_analog_input_dead_band(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::AnalogInputDeadBand, u16)>,
    ) {
        self.push(Kind::AnalogInputDeadBand, info, iter)
    }

    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::AnalogOutputStatus, u16)>,
    ) {
        self.push(Kind::AnalogOutputStatus, info, iter)
    }

    fn handle_analog_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::AnalogOutputCommandEvent, u16)>,
    ) {
        self.push(Kind::AnalogOutputCommandEvent, info, iter)
    }

    fn handle_binary_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::BinaryOutputCommandEvent, u16)>,
    ) {
        self.push(Kind::BinaryOutputCommandEvent, info, iter)
    }

    fn handle_unsigned_integer(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (m::UnsignedInteger, u16)>,
    ) {
        self.push(Kind::UnsignedInteger, info, iter)
    }

    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        self.push(Kind::OctetString, info, iter)
    }

    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {
        let set = match attr {
            AnyAttribute::Other(x) => x.set,
            AnyAttribute::Known(_) => AttrSet::Default,
        };
        let index = match set {
            AttrSet::Default => 0,
            AttrSet::Private(x) => x.into(),
        };
        self.calls.push(Call {
            kind: Kind::DeviceAttribute,
            info,
            indices: vec![index],
        });
    }

    fn handle_abs_time(&mut self, info: HeaderInfo, _time: Timestamp) {
        self.calls.push(Call {
            kind: Kind::AbsTime,
            info,
            indices: Vec::new(),
        });
    }
}

/// A common time of occurrence, so that objects with relative time can be extracted
const CTO: [u8; 10] = [51, 1, 0x07, 1, 1, 2, 3, 4, 5, 6];

/// The calls, and whether extraction reports values, for response objects
fn extract(options: ParseOptions, objects: &[u8]) -> (Vec<Call>, bool) {
    let objects = HeaderCollection::parse(options, FunctionCode::Response, objects).unwrap();
    let mut recorder = Recorder::default();
    let delivered = extract_measurements_inner(objects, &mut recorder);
    (recorder.calls, delivered)
}

/// The qualifiers of a variation that master extraction passes to the handler
#[derive(Copy, Clone)]
enum Family {
    Range,
    Count,
    Prefixed,
}

impl Family {
    fn contains(self, qualifier: QualifierCode) -> bool {
        match self {
            Family::Range => matches!(qualifier, Q::Range8 | Q::Range16),
            Family::Count => matches!(qualifier, Q::Count8 | Q::Count16),
            Family::Prefixed => matches!(qualifier, Q::CountAndPrefix8 | Q::CountAndPrefix16),
        }
    }
}

use Family::{Count as C, Prefixed as P, Range as R};
use Kind::*;

/// Each variation outside groups 0, 110 and 111 that master extraction passes to the handler,
/// with the callback, whether `HeaderInfo` says it's an event and has flags, and its qualifiers
const EXTRACTED: &[(Variation, Kind, bool, bool, Family)] = &[
    (Variation::Group1Var1, BinaryInput, false, false, R),
    (Variation::Group1Var2, BinaryInput, false, true, R),
    (Variation::Group2Var1, BinaryInput, true, true, P),
    (Variation::Group2Var2, BinaryInput, true, true, P),
    (Variation::Group2Var3, BinaryInput, true, true, P),
    (Variation::Group3Var1, DoubleBitBinaryInput, false, false, R),
    (Variation::Group3Var2, DoubleBitBinaryInput, false, true, R),
    (Variation::Group4Var1, DoubleBitBinaryInput, true, true, P),
    (Variation::Group4Var2, DoubleBitBinaryInput, true, true, P),
    (Variation::Group4Var3, DoubleBitBinaryInput, true, true, P),
    (Variation::Group10Var1, BinaryOutputStatus, false, false, R),
    (Variation::Group10Var2, BinaryOutputStatus, false, true, R),
    (Variation::Group11Var1, BinaryOutputStatus, true, true, P),
    (Variation::Group11Var2, BinaryOutputStatus, true, true, P),
    (
        Variation::Group13Var1,
        BinaryOutputCommandEvent,
        true,
        true,
        P,
    ),
    (
        Variation::Group13Var2,
        BinaryOutputCommandEvent,
        true,
        true,
        P,
    ),
    (Variation::Group20Var1, Counter, false, true, R),
    (Variation::Group20Var2, Counter, false, true, R),
    (Variation::Group20Var5, Counter, false, false, R),
    (Variation::Group20Var6, Counter, false, false, R),
    (Variation::Group21Var1, FrozenCounter, false, true, R),
    (Variation::Group21Var2, FrozenCounter, false, true, R),
    (Variation::Group21Var5, FrozenCounter, false, true, R),
    (Variation::Group21Var6, FrozenCounter, false, true, R),
    (Variation::Group21Var9, FrozenCounter, false, false, R),
    (Variation::Group21Var10, FrozenCounter, false, false, R),
    (Variation::Group22Var1, Counter, true, true, P),
    (Variation::Group22Var2, Counter, true, true, P),
    (Variation::Group22Var5, Counter, true, true, P),
    (Variation::Group22Var6, Counter, true, true, P),
    (Variation::Group23Var1, FrozenCounter, true, true, P),
    (Variation::Group23Var2, FrozenCounter, true, true, P),
    (Variation::Group23Var5, FrozenCounter, true, true, P),
    (Variation::Group23Var6, FrozenCounter, true, true, P),
    (Variation::Group30Var1, AnalogInput, false, true, R),
    (Variation::Group30Var2, AnalogInput, false, true, R),
    (Variation::Group30Var3, AnalogInput, false, false, R),
    (Variation::Group30Var4, AnalogInput, false, false, R),
    (Variation::Group30Var5, AnalogInput, false, true, R),
    (Variation::Group30Var6, AnalogInput, false, true, R),
    (Variation::Group31Var1, FrozenAnalogInput, false, true, R),
    (Variation::Group31Var2, FrozenAnalogInput, false, true, R),
    (Variation::Group31Var3, FrozenAnalogInput, false, true, R),
    (Variation::Group31Var4, FrozenAnalogInput, false, true, R),
    (Variation::Group31Var5, FrozenAnalogInput, false, false, R),
    (Variation::Group31Var6, FrozenAnalogInput, false, false, R),
    (Variation::Group31Var7, FrozenAnalogInput, false, true, R),
    (Variation::Group31Var8, FrozenAnalogInput, false, true, R),
    (Variation::Group32Var1, AnalogInput, true, true, P),
    (Variation::Group32Var2, AnalogInput, true, true, P),
    (Variation::Group32Var3, AnalogInput, true, true, P),
    (Variation::Group32Var4, AnalogInput, true, true, P),
    (Variation::Group32Var5, AnalogInput, true, true, P),
    (Variation::Group32Var6, AnalogInput, true, true, P),
    (Variation::Group32Var7, AnalogInput, true, true, P),
    (Variation::Group32Var8, AnalogInput, true, true, P),
    (Variation::Group33Var1, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var2, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var3, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var4, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var5, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var6, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var7, FrozenAnalogInput, true, true, P),
    (Variation::Group33Var8, FrozenAnalogInput, true, true, P),
    (Variation::Group34Var1, AnalogInputDeadBand, false, false, R),
    (Variation::Group34Var2, AnalogInputDeadBand, false, false, R),
    (Variation::Group34Var3, AnalogInputDeadBand, false, false, R),
    (Variation::Group40Var1, AnalogOutputStatus, false, true, R),
    (Variation::Group40Var2, AnalogOutputStatus, false, true, R),
    (Variation::Group40Var3, AnalogOutputStatus, false, true, R),
    (Variation::Group40Var4, AnalogOutputStatus, false, true, R),
    (Variation::Group42Var1, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var2, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var3, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var4, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var5, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var6, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var7, AnalogOutputStatus, true, true, P),
    (Variation::Group42Var8, AnalogOutputStatus, true, true, P),
    (
        Variation::Group43Var1,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var2,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var3,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var4,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var5,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var6,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var7,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (
        Variation::Group43Var8,
        AnalogOutputCommandEvent,
        true,
        false,
        P,
    ),
    (Variation::Group50Var1, AbsTime, false, false, C),
    (Variation::Group102Var1, UnsignedInteger, false, false, R),
];

/// Attributes of the default set whose type isn't a visible string, so extraction drops the
/// visible string of the probe
fn not_visible_string(var: u8) -> bool {
    matches!(var, 198..=200 | 203..=205 | 209 | 210 | 212..=241 | 255)
}

fn info(
    variation: Variation,
    qualifier: QualifierCode,
    is_event: bool,
    has_flags: bool,
) -> HeaderInfo {
    HeaderInfo {
        variation,
        qualifier,
        is_event,
        has_flags,
    }
}

/// The calls that extraction makes for a header of one object at index 0
fn expected_calls(variation: Variation, qualifier: QualifierCode) -> Vec<Call> {
    let row = match variation {
        Variation::Group0(var) if not_visible_string(var) => None,
        // an attribute in a range or count and prefix header, in set 0
        Variation::Group0(_) => Some((DeviceAttribute, false, false, qualifier != Q::AllObjects)),
        Variation::Group110(_) => Some((OctetString, false, false, R.contains(qualifier))),
        Variation::Group111(_) => Some((OctetString, true, false, P.contains(qualifier))),
        _ => EXTRACTED.iter().find(|row| row.0 == variation).map(
            |&(_, kind, is_event, has_flags, family)| {
                (kind, is_event, has_flags, family.contains(qualifier))
            },
        ),
    };
    let Some((kind, is_event, has_flags, true)) = row else {
        return Vec::new();
    };
    let indices = match kind {
        AbsTime => Vec::new(),
        _ => vec![0],
    };
    vec![Call {
        kind,
        info: info(variation, qualifier, is_event, has_flags),
        indices,
    }]
}

#[test]
fn every_response_header_passes_this_header_info() {
    for options in [ParseOptions::default(), ParseOptions::parse_everything()] {
        for (variation, qualifier, bytes) in accepted_headers(options, FunctionCode::Response) {
            let expected = expected_calls(variation, qualifier);
            let delivered = !expected.is_empty();
            assert_eq!(
                extract(options, &[&CTO[..], &bytes].concat()),
                (expected, delivered),
                "{variation:?} {qualifier:?}"
            );
        }
    }
}

#[test]
fn headers_of_several_objects_pass_header_info_once_with_every_index() {
    let objects: &[&[u8]] = &[
        // g1v1 packed binary inputs, 8-bit start 0 and stop 9
        &[1, 1, 0x00, 0, 9, 0xFF, 0x03],
        // g3v1 packed double-bit binary inputs, 16-bit start 1 and stop 5
        &[3, 1, 0x01, 1, 0, 5, 0, 0x55, 0x01],
        // g10v1 packed binary output status, 8-bit start 3 and stop 4
        &[10, 1, 0x00, 3, 4, 0x03],
        // g51v2 unsynchronized common time of occurrence
        &[51, 2, 0x07, 1, 1, 2, 3, 4, 5, 6],
        // g2v3 binary input events with relative time, 8-bit count of 2 and indices 7 and 9
        &[2, 3, 0x17, 2, 7, 0x81, 1, 0, 9, 0x01, 2, 0],
        // g4v3 double-bit binary input events with relative time, 16-bit count of 1 and index 300
        &[4, 3, 0x28, 1, 0, 0x2C, 0x01, 0x41, 3, 0],
        // g110v2 octet strings of 2 bytes, 8-bit start 5 and stop 6
        &[110, 2, 0x00, 5, 6, 0xAA, 0xBB, 0xCC, 0xDD],
        // g111v3 octet string events of 3 bytes, 8-bit count of 2 and indices 4 and 2
        &[111, 3, 0x17, 2, 4, 1, 2, 3, 2, 4, 5, 6],
        // g0v1 visible string attribute in private set 5
        &[0, 1, 0x00, 5, 5, 1, 1, b'A'],
        // g50v1 absolute time, 8-bit count of 1
        &[50, 1, 0x07, 1, 1, 2, 3, 4, 5, 6],
    ];
    let expected = vec![
        Call {
            kind: BinaryInput,
            info: info(Variation::Group1Var1, Q::Range8, false, false),
            indices: (0..=9).collect(),
        },
        Call {
            kind: DoubleBitBinaryInput,
            info: info(Variation::Group3Var1, Q::Range16, false, false),
            indices: (1..=5).collect(),
        },
        Call {
            kind: BinaryOutputStatus,
            info: info(Variation::Group10Var1, Q::Range8, false, false),
            indices: vec![3, 4],
        },
        Call {
            kind: BinaryInput,
            info: info(Variation::Group2Var3, Q::CountAndPrefix8, true, true),
            indices: vec![7, 9],
        },
        Call {
            kind: DoubleBitBinaryInput,
            info: info(Variation::Group4Var3, Q::CountAndPrefix16, true, true),
            indices: vec![300],
        },
        Call {
            kind: OctetString,
            info: info(Variation::Group110(2), Q::Range8, false, false),
            indices: vec![5, 6],
        },
        Call {
            kind: OctetString,
            info: info(Variation::Group111(3), Q::CountAndPrefix8, true, false),
            indices: vec![4, 2],
        },
        Call {
            kind: DeviceAttribute,
            info: info(Variation::Group0(1), Q::Range8, false, false),
            indices: vec![5],
        },
        Call {
            kind: AbsTime,
            info: info(Variation::Group50Var1, Q::Count8, false, false),
            indices: Vec::new(),
        },
    ];
    assert_eq!(
        extract(ParseOptions::default(), &objects.concat()),
        (expected, true)
    );
}
