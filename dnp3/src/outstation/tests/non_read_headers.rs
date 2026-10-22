//! Pins the IIN2 bits and the application calls of each non-READ function for every header the
//! parser accepts outside a READ: each variation and qualifier is either handled, rejected with
//! PARAMETER_ERROR because the function supports the variation with another qualifier, or rejected
//! with OBJECT_UNKNOWN
//!
//! A range header names index 0, and a count or a count and prefix header has one object, at
//! index 0, of zeros (see `accepted_headers`).

use crate::app::attr::{var, AttrProp, AttrSet, StringAttr};
use crate::app::parse::header_identity_tests::accepted_headers;
use crate::app::parse::options::ParseOptions;
use crate::app::{FunctionCode, Iin2, QualifierCode, Timestamp, Variation};
use crate::outstation::database::{Add, AnalogInputConfig};
use crate::outstation::{FreezeIndices, FreezeInterval, FreezeType};

use super::harness::*;

use QualifierCode as Q;
use Variation as V;

const OPTIONS: ParseOptions = ParseOptions {
    parse_zero_length_strings: false,
};

const WRITE: u8 = 0x02;
const SELECT: u8 = 0x03;
const OPERATE: u8 = 0x04;
const DIRECT_OPERATE: u8 = 0x05;
const DIRECT_OPERATE_NO_ACK: u8 = 0x06;
const IMMEDIATE_FREEZE: u8 = 0x07;
const IMMEDIATE_FREEZE_NO_ACK: u8 = 0x08;
const FREEZE_CLEAR: u8 = 0x09;
const FREEZE_CLEAR_NO_ACK: u8 = 0x0A;
const FREEZE_AT_TIME: u8 = 0x0B;
const FREEZE_AT_TIME_NO_ACK: u8 = 0x0C;
const ENABLE_UNSOLICITED: u8 = 0x14;
const DISABLE_UNSOLICITED: u8 = 0x15;
const RECORD_CURRENT_TIME: u8 = 0x18;

/// g50v2 with a time and an interval of zero
const FREEZE_TIME: &[u8] = &[50, 2, 0x07, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

fn responds(function: u8) -> bool {
    !matches!(
        function,
        DIRECT_OPERATE_NO_ACK
            | IMMEDIATE_FREEZE_NO_ACK
            | FREEZE_CLEAR_NO_ACK
            | FREEZE_AT_TIME_NO_ACK
    )
}

/// What the outstation does with a request
#[derive(Debug, PartialEq)]
struct Answer {
    /// the IIN2 bits of the response, or `None` if there's no response
    iin2: Option<Iin2>,
    /// the application and control handler calls
    events: Vec<Event>,
}

/// What a function should do with a header
enum Expected {
    /// the function handles the header with this IIN2 and these calls
    Handled(Iin2, Vec<Event>),
    /// the function rejects the header with this IIN2, without any call
    Rejected(Iin2),
}

impl Expected {
    fn ok(events: Vec<Event>) -> Self {
        Self::Handled(Iin2::default(), events)
    }

    /// PARAMETER_ERROR if the function supports the variation with another qualifier, otherwise
    /// OBJECT_UNKNOWN
    fn unsupported(supports_variation: bool) -> Self {
        Self::Rejected(if supports_variation {
            Iin2::PARAMETER_ERROR
        } else {
            Iin2::OBJECT_UNKNOWN
        })
    }
}

/// The time written by g50v3 depends on the clock, so every written time compares as zero
fn normalize(event: Event) -> Event {
    match event {
        Event::WriteAbsoluteTime(_) => Event::WriteAbsoluteTime(Timestamp::new(0)),
        x => x,
    }
}

/// Sends requests with increasing sequence numbers so none is taken for a repeat
struct Requester {
    harness: OutstationHarness,
    seq: u8,
}

impl Requester {
    fn new(harness: OutstationHarness) -> Self {
        Self { harness, seq: 0 }
    }

    async fn send(&mut self, function: u8, objects: &[u8]) -> Answer {
        let control = 0xC0 | self.seq;
        self.seq = self.seq.wrapping_add(1) & 0x0F;
        let request = [&[control, function], objects].concat();
        self.harness.send_and_process(&request).await;
        let iin2 = if responds(function) {
            let response = self.harness.expect_write().await;
            assert_eq!(response[0], control);
            assert_eq!(response[1], 0x81);
            Some(Iin2::new(response[3]))
        } else {
            self.harness.expect_no_response();
            None
        };
        let mut events = Vec::new();
        while let Some(event) = self.harness.poll_event() {
            events.push(normalize(event));
        }
        Answer { iin2, events }
    }
}

/// Check the answer of a function to each header the parser accepts, after the objects in
/// `before`, and return the variation and qualifier of the headers it handles
///
/// Before each request, `prelude` is sent and must be answered without IIN2 bits or calls.
async fn check(
    requester: &mut Requester,
    prelude: Option<u8>,
    function: u8,
    before: &[u8],
    expected: impl Fn(Variation, QualifierCode) -> Expected,
) -> Vec<(Variation, QualifierCode)> {
    let mut handled = Vec::new();
    for (variation, qualifier, bytes) in accepted_headers(OPTIONS, FunctionCode::Write) {
        if let Some(prelude) = prelude {
            assert_eq!(
                requester.send(prelude, &[]).await,
                Answer {
                    iin2: Some(Iin2::default()),
                    events: Vec::new()
                }
            );
        }
        let (iin2, events) = match expected(variation, qualifier) {
            Expected::Handled(iin2, events) => {
                handled.push((variation, qualifier));
                (iin2, events)
            }
            Expected::Rejected(iin2) => (iin2, Vec::new()),
        };
        let expected = Answer {
            iin2: responds(function).then_some(iin2),
            events,
        };
        let actual = requester.send(function, &[before, &bytes].concat()).await;
        assert_eq!(
            actual, expected,
            "function {function:#04X} {variation:?} {qualifier:?}"
        );
    }
    handled
}

fn is_range(qualifier: QualifierCode) -> bool {
    matches!(qualifier, Q::Range8 | Q::Range16)
}

fn is_count(qualifier: QualifierCode) -> bool {
    matches!(qualifier, Q::Count8 | Q::Count16)
}

fn is_prefixed(qualifier: QualifierCode) -> bool {
    matches!(qualifier, Q::CountAndPrefix8 | Q::CountAndPrefix16)
}

fn is_writable(variation: Variation) -> bool {
    match variation {
        V::Group0(var) => var != 255,
        V::Group34Var1
        | V::Group34Var2
        | V::Group34Var3
        | V::Group50Var1
        | V::Group50Var3
        | V::Group80Var1 => true,
        _ => false,
    }
}

fn write(variation: Variation, qualifier: QualifierCode) -> Expected {
    match variation {
        // only the defined attribute can be written
        V::Group0(var) if var != 255 && is_range(qualifier) => {
            if var == var::USER_ASSIGNED_LOCATION {
                Expected::ok(vec![Event::WriteDeviceAttr(AttrSet::Default, var)])
            } else {
                Expected::Handled(Iin2::OBJECT_UNKNOWN, Vec::new())
            }
        }
        // IIN index 0 can't be written
        V::Group80Var1 if is_range(qualifier) => {
            Expected::Handled(Iin2::PARAMETER_ERROR, Vec::new())
        }
        V::Group50Var1 | V::Group50Var3 if is_count(qualifier) => {
            Expected::ok(vec![Event::WriteAbsoluteTime(Timestamp::new(0))])
        }
        V::Group34Var1 | V::Group34Var2 | V::Group34Var3
            if is_range(qualifier) || is_prefixed(qualifier) =>
        {
            Expected::ok(vec![
                Event::BeginWriteDeadBands,
                Event::WriteDeadBand(0, 0.0),
                Event::EndWriteDeadBands,
            ])
        }
        _ => Expected::unsupported(is_writable(variation)),
    }
}

#[tokio::test]
async fn write_answers_each_variation_and_qualifier() {
    let harness = new_harness(get_default_config());
    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::writable(),
            StringAttr::UserAssignedLocation.with_value("Bend"),
        )
        .unwrap();
        db.add(0, None, AnalogInputConfig::default());
    });
    let mut requester = Requester::new(harness);

    // record the current time before each request so that g50v3 can be written
    let mut handled = check(&mut requester, Some(RECORD_CURRENT_TIME), WRITE, &[], write).await;

    let range = |v| [(v, Q::Range8), (v, Q::Range16)];
    let count = |v| [(v, Q::Count8), (v, Q::Count16)];
    let prefixed = |v| [(v, Q::CountAndPrefix8), (v, Q::CountAndPrefix16)];
    let mut expected: Vec<_> = (1..=253).flat_map(|var| range(V::Group0(var))).collect();
    for v in [V::Group34Var1, V::Group34Var2, V::Group34Var3] {
        expected.extend(range(v));
        expected.extend(prefixed(v));
    }
    expected.extend(range(V::Group80Var1));
    expected.extend(count(V::Group50Var1));
    expected.extend(count(V::Group50Var3));
    expected.sort_by_key(|(v, q)| (v.to_group_and_var(), q.as_u8()));
    handled.sort_by_key(|(v, q)| (v.to_group_and_var(), q.as_u8()));
    assert_eq!(handled, expected);
}

fn unsolicited_class(variation: Variation) -> bool {
    matches!(variation, V::Group60Var2 | V::Group60Var3 | V::Group60Var4)
}

fn enable_or_disable_unsolicited(variation: Variation, qualifier: QualifierCode) -> Expected {
    if unsolicited_class(variation) && qualifier == Q::AllObjects {
        return Expected::ok(Vec::new());
    }
    Expected::unsupported(unsolicited_class(variation))
}

#[tokio::test]
async fn enable_and_disable_unsolicited_answer_each_variation_and_qualifier() {
    let mut harness = new_harness(get_default_unsolicited_config());
    // confirm the null unsolicited response
    harness.expect_response(&[0xF0, 0x82, 0x80, 0x00]).await;
    harness.send_and_process(&[0xD0, 0x00]).await;
    harness.check_events(&[
        Event::EnterUnsolicitedConfirmWait(0),
        Event::UnsolicitedConfirmReceived(0),
    ]);
    let mut requester = Requester::new(harness);

    for function in [ENABLE_UNSOLICITED, DISABLE_UNSOLICITED] {
        let handled = check(
            &mut requester,
            None,
            function,
            &[],
            enable_or_disable_unsolicited,
        )
        .await;
        assert_eq!(
            handled,
            [
                (V::Group60Var2, Q::AllObjects),
                (V::Group60Var3, Q::AllObjects),
                (V::Group60Var4, Q::AllObjects),
            ]
        );
    }
}

#[tokio::test]
async fn enable_and_disable_unsolicited_without_unsolicited_support_answer_no_func_code_support() {
    let mut requester = Requester::new(new_harness(get_default_config()));

    for function in [ENABLE_UNSOLICITED, DISABLE_UNSOLICITED] {
        check(&mut requester, None, function, &[], |_, _| {
            Expected::Rejected(Iin2::NO_FUNC_CODE_SUPPORT)
        })
        .await;
    }
}

fn freeze(variation: Variation, qualifier: QualifierCode, freeze_type: FreezeType) -> Expected {
    let indices = match (variation, qualifier) {
        (V::Group20Var0, Q::AllObjects) => FreezeIndices::All,
        (V::Group20Var0, Q::Range8 | Q::Range16) => FreezeIndices::Range(0, 0),
        _ => return Expected::unsupported(variation == V::Group20Var0),
    };
    Expected::ok(vec![Event::Freeze(indices, freeze_type)])
}

const FREEZE_COUNTERS: [(Variation, QualifierCode); 3] = [
    (V::Group20Var0, Q::AllObjects),
    (V::Group20Var0, Q::Range8),
    (V::Group20Var0, Q::Range16),
];

const FREEZE_TIMES: [(Variation, QualifierCode); 2] =
    [(V::Group50Var2, Q::Count8), (V::Group50Var2, Q::Count16)];

#[tokio::test]
async fn freeze_functions_answer_each_variation_and_qualifier() {
    let mut requester = Requester::new(new_harness(get_default_config()));

    for (function, freeze_type) in [
        (IMMEDIATE_FREEZE, FreezeType::ImmediateFreeze),
        (IMMEDIATE_FREEZE_NO_ACK, FreezeType::ImmediateFreeze),
        (FREEZE_CLEAR, FreezeType::FreezeAndClear),
        (FREEZE_CLEAR_NO_ACK, FreezeType::FreezeAndClear),
    ] {
        let handled = check(&mut requester, None, function, &[], |v, q| {
            freeze(v, q, freeze_type)
        })
        .await;
        assert_eq!(handled, FREEZE_COUNTERS);
    }
}

fn is_freeze_time(variation: Variation, qualifier: QualifierCode) -> bool {
    variation == V::Group50Var2 && is_count(qualifier)
}

/// Without a preceding g50v2, every header other than a g50v2 is a PARAMETER_ERROR
fn freeze_at_time_without_time(variation: Variation, qualifier: QualifierCode) -> Expected {
    if is_freeze_time(variation, qualifier) {
        Expected::ok(Vec::new())
    } else {
        Expected::Rejected(Iin2::PARAMETER_ERROR)
    }
}

/// After a g50v2, a header is either another g50v2 or a freeze of counters at that time
fn freeze_at_time(variation: Variation, qualifier: QualifierCode) -> Expected {
    if is_freeze_time(variation, qualifier) {
        return Expected::ok(Vec::new());
    }
    freeze(
        variation,
        qualifier,
        FreezeType::FreezeAtTime(FreezeInterval::new(Timestamp::new(0), 0)),
    )
}

#[tokio::test]
async fn freeze_at_time_answers_each_variation_and_qualifier() {
    let mut requester = Requester::new(new_harness(get_default_config()));

    for function in [FREEZE_AT_TIME, FREEZE_AT_TIME_NO_ACK] {
        let handled = check(
            &mut requester,
            None,
            function,
            &[],
            freeze_at_time_without_time,
        )
        .await;
        assert_eq!(handled, FREEZE_TIMES);

        let handled = check(&mut requester, None, function, FREEZE_TIME, freeze_at_time).await;
        assert_eq!(
            handled,
            [FREEZE_COUNTERS.as_slice(), &FREEZE_TIMES].concat()
        );
    }
}

const CONTROLS: [Variation; 5] = [
    V::Group12Var1,
    V::Group41Var1,
    V::Group41Var2,
    V::Group41Var3,
    V::Group41Var4,
];

/// A control function rejects any object other than a control with OBJECT_UNKNOWN; the parser
/// accepts the controls only with a count and prefix qualifier
#[tokio::test]
async fn control_functions_answer_each_variation_and_qualifier() {
    let mut requester = Requester::new(new_harness(get_default_config()));

    for function in [SELECT, OPERATE, DIRECT_OPERATE, DIRECT_OPERATE_NO_ACK] {
        let mut handled = Vec::new();
        for (variation, qualifier, bytes) in accepted_headers(OPTIONS, FunctionCode::Write) {
            let Answer { iin2, events } = requester.send(function, &bytes).await;
            let expected = if CONTROLS.contains(&variation) && is_prefixed(qualifier) {
                handled.push((variation, qualifier));
                // OPERATE without a SELECT doesn't reach the handler
                assert_eq!(
                    events.first() == Some(&Event::BeginControls),
                    function != OPERATE,
                    "function {function:#04X} {variation:?} {qualifier:?} {events:?}"
                );
                Iin2::default()
            } else {
                assert_eq!(events, Vec::new());
                Iin2::OBJECT_UNKNOWN
            };
            assert_eq!(
                iin2,
                responds(function).then_some(expected),
                "function {function:#04X} {variation:?} {qualifier:?}"
            );
        }
        let expected: Vec<_> = [Q::CountAndPrefix8, Q::CountAndPrefix16]
            .into_iter()
            .flat_map(|q| CONTROLS.map(|v| (v, q)))
            .collect();
        assert_eq!(handled, expected);
    }
}
