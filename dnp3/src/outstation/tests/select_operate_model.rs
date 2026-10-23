// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Select-before-operate against a model of IEEE 1815-2012 4.4.4.3 and Table 4-9
//!
//! The model keeps its own selection, last request and clock, and predicts the response bytes and
//! control callbacks of each request from the request's own encoding. Where the standard allows
//! more than one outcome the model offers each, and continues from the one the outstation chose:
//! - an OPERATE that matches the selection exactly at the select timeout may execute or time out
//!   (Rule 1 doesn't say which side of the boundary the expiry is on);
//! - an OPERATE that doesn't match an expired selection may get NO_SELECT or TIMEOUT (Rule 1
//!   could be read as rejecting any later OPERATE with TIMEOUT);
//! - a rejected OPERATE of two objects may echo both objects or only the first (Rule 6).
//!
//! A malformed SELECT with the selection's sequence number isn't the discarded SELECT of Table
//! 4-9 row 2: a request whose objects don't parse is still a request, answered with IIN2.2, which
//! replaces the last request and ends the selection (decision 0003 in the protocol reference,
//! TB2016-001 4.5.11).
//!
//! A CONFIRM keeps the selection: Table 4-2 classes it as a confirmation, not a request, so it
//! isn't the "next request" of Rule 4. So does a fragment that isn't a valid request (FIR=0).
//!
//! Besides single requests, a run sends a SELECT, one request or clock advance, and the OPERATE
//! that matches the SELECT, so each ruling on whether a request ends the selection is followed by
//! the OPERATE that shows it.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the model adds small sequence numbers and millisecond counts"
)]

use std::future::Future;

use proptest::prelude::*;
use tokio::time::{Duration, Instant};

use crate::app::control::{
    CommandStatus, ControlCode, Group12Var1, Group41Var2, OpType, TripCloseCode,
};
use crate::app::{FunctionCode, Timeout};
use crate::outstation::config::OutstationConfig;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::OperateType;
use crate::util::test_support::watchdog;

/// the index whose control handler status the tests change
const STATUS_INDEX: u16 = 3;

/// an object in a control request
#[derive(Copy, Clone, Debug, PartialEq)]
enum Obj {
    /// g41v2 with the request's status code
    G41V2 { index: u16, value: i16, status: u8 },
    /// g12v1 close/pulse-on, count 1, on 100 ms, off 200 ms
    Crob { index: u16 },
}

impl Obj {
    fn index(self) -> u16 {
        match self {
            Obj::G41V2 { index, .. } => index,
            Obj::Crob { index } => index,
        }
    }

    /// the object after its index, with `status` in place of its status code
    fn write_body(self, status: u8, out: &mut Vec<u8>) {
        match self {
            Obj::G41V2 { value, .. } => {
                out.extend_from_slice(&value.to_le_bytes());
                out.push(status);
            }
            Obj::Crob { .. } => {
                out.extend_from_slice(&[0x41, 0x01]);
                out.extend_from_slice(&100u32.to_le_bytes());
                out.extend_from_slice(&200u32.to_le_bytes());
                out.push(status);
            }
        }
    }

    fn request_status(self) -> u8 {
        match self {
            Obj::G41V2 { status, .. } => status,
            Obj::Crob { .. } => 0,
        }
    }

    /// the control as the handler receives it
    fn control(self) -> Control {
        match self {
            Obj::G41V2 {
                index,
                value,
                status,
            } => Control::G41V2(
                Group41Var2 {
                    value,
                    status: CommandStatus::from(status),
                },
                index,
            ),
            Obj::Crob { index } => Control::G12V1(
                Group12Var1 {
                    code: ControlCode::from_tcc_and_op_type(TripCloseCode::Close, OpType::PulseOn),
                    count: 1,
                    on_time: 100,
                    off_time: 200,
                    status: CommandStatus::Success,
                },
                index,
            ),
        }
    }
}

/// one header of controls: a 1-byte (0x17) or 2-byte (0x28) count and index prefix
#[derive(Copy, Clone, Debug, PartialEq)]
struct ObjectSet {
    two_byte_prefix: bool,
    objects: &'static [Obj],
}

const fn g41v2(index: u16, value: i16, status: u8) -> Obj {
    Obj::G41V2 {
        index,
        value,
        status,
    }
}

/// the sets with the object whose handler status changes first and last
const FAILING_FIRST: usize = 8;
const FAILING_LAST: usize = 9;

/// the set with one object at index 0
const INDEX_0: usize = 10;

/// the first set, and sets that each differ from it in one way, plus the CROB whose handler
/// status changes, two sets with another object at that index, first and last, and an object at
/// index 0
const SETS: [ObjectSet; 11] = [
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(7, 513, 0)],
    },
    // value
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(7, 514, 0)],
    },
    // index
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(8, 513, 0)],
    },
    // qualifier
    ObjectSet {
        two_byte_prefix: true,
        objects: &[g41v2(7, 513, 0)],
    },
    // status code in the request
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(7, 513, 1)],
    },
    // two objects
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(7, 513, 0), g41v2(8, 513, 0)],
    },
    // the same two objects in the other order
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(8, 513, 0), g41v2(7, 513, 0)],
    },
    ObjectSet {
        two_byte_prefix: false,
        objects: &[Obj::Crob {
            index: STATUS_INDEX,
        }],
    },
    // Rule 3: the selection fails if any object fails, wherever it is
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(STATUS_INDEX, 513, 0), g41v2(7, 513, 0)],
    },
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(7, 513, 0), g41v2(STATUS_INDEX, 513, 0)],
    },
    ObjectSet {
        two_byte_prefix: false,
        objects: &[g41v2(0, 513, 0)],
    },
];

impl ObjectSet {
    fn write_prefix(&self, value: u16, out: &mut Vec<u8>) {
        if self.two_byte_prefix {
            out.extend_from_slice(&value.to_le_bytes());
        } else {
            out.push(u8::try_from(value).unwrap());
        }
    }

    /// the header and the first `count` objects, each with the status that `status` gives it
    fn write(&self, count: usize, status: impl Fn(Obj) -> u8, out: &mut Vec<u8>) {
        let (group, variation) = match self.objects[0] {
            Obj::G41V2 { .. } => (41, 2),
            Obj::Crob { .. } => (12, 1),
        };
        let qualifier = if self.two_byte_prefix { 0x28 } else { 0x17 };
        out.extend_from_slice(&[group, variation, qualifier]);
        self.write_prefix(u16::try_from(count).unwrap(), out);
        for obj in &self.objects[..count] {
            self.write_prefix(obj.index(), out);
            obj.write_body(status(*obj), out);
        }
    }

    fn request_objects(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write(self.objects.len(), Obj::request_status, &mut out);
        out
    }
}

/// a request the test sends, with its sequence number resolved
#[derive(Copy, Clone, Debug, PartialEq)]
enum Request {
    Select(usize, u8),
    Operate(usize, u8),
    DirectOperate(usize, u8),
    DirectOperateNoAck(usize, u8),
    /// READ class 0
    Read(u8),
    DelayMeasure(u8),
    /// a SELECT or OPERATE of the first set without its status code, or a READ whose objects end
    /// early
    Malformed(FunctionCode, u8),
    UnknownFunction(u8),
    Confirm {
        uns: bool,
        seq: u8,
    },
    /// READ class 0 with FIR = 0: not a valid request
    NotFirst(u8),
}

const UNKNOWN_FUNCTION: u8 = 0x70;

impl Request {
    fn bytes(self) -> Vec<u8> {
        let control = |function: FunctionCode, set: usize, seq: u8| {
            let mut bytes = vec![0xC0 | seq, function.as_u8()];
            bytes.extend(SETS[set].request_objects());
            bytes
        };
        match self {
            Request::Select(set, seq) => control(FunctionCode::Select, set, seq),
            Request::Operate(set, seq) => control(FunctionCode::Operate, set, seq),
            Request::DirectOperate(set, seq) => control(FunctionCode::DirectOperate, set, seq),
            Request::DirectOperateNoAck(set, seq) => {
                control(FunctionCode::DirectOperateNoResponse, set, seq)
            }
            Request::Read(seq) => read_class_0(seq).to_vec(),
            Request::DelayMeasure(seq) => delay_measure(seq).to_vec(),
            Request::Malformed(FunctionCode::Read, seq) => vec![0xC0 | seq, 0x01, 60],
            Request::Malformed(function, seq) => {
                let mut bytes = control(function, 0, seq);
                bytes.pop();
                bytes
            }
            Request::UnknownFunction(seq) => vec![0xC0 | seq, UNKNOWN_FUNCTION],
            Request::Confirm { uns: false, seq } => confirm(seq).to_vec(),
            Request::Confirm { uns: true, seq } => unsolicited_confirm(seq).to_vec(),
            Request::NotFirst(seq) => vec![0x40 | seq, 0x01, 60, 1, 0x06],
        }
    }

    fn seq(self) -> u8 {
        match self {
            Request::Select(_, seq)
            | Request::Operate(_, seq)
            | Request::DirectOperate(_, seq)
            | Request::DirectOperateNoAck(_, seq)
            | Request::Read(seq)
            | Request::DelayMeasure(seq)
            | Request::Malformed(_, seq)
            | Request::UnknownFunction(seq)
            | Request::Confirm { seq, .. }
            | Request::NotFirst(seq) => seq,
        }
    }
}

/// the response header with the restart IIN, which no request clears
fn response_header(seq: u8, iin2: u8) -> Vec<u8> {
    vec![0xC0 | seq, 0x81, 0x80, iin2]
}

/// a response that echoes the first `count` objects of `set`, each with the status `status` gives
fn echo(seq: u8, set: usize, count: usize, status: impl Fn(Obj) -> u8) -> Vec<u8> {
    let mut bytes = response_header(seq, 0);
    SETS[set].write(count, status, &mut bytes);
    bytes
}

/// what the outstation does in answer to one input
#[derive(Clone, Debug, PartialEq)]
struct Outcome {
    response: Option<Vec<u8>>,
    events: Vec<Event>,
}

impl Outcome {
    fn nothing() -> Self {
        Self {
            response: None,
            events: Vec::new(),
        }
    }

    fn response(response: Vec<u8>) -> Self {
        Self {
            response: Some(response),
            events: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Selection {
    seq: u8,
    request: Vec<u8>,
    set: usize,
    at_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
struct LastRequest {
    bytes: Vec<u8>,
    response: Option<Vec<u8>>,
}

#[derive(Clone, Debug)]
struct Model {
    select_timeout_ms: u64,
    now_ms: u64,
    selection: Option<Selection>,
    last_request: Option<LastRequest>,
    /// the status the handler returns for [`STATUS_INDEX`]
    handler_status: CommandStatus,
    /// what ops resolve against: the last SELECT's sequence number and objects, whether or not
    /// it selected them, and the last request's sequence number
    last_select_seq: u8,
    last_select_set: usize,
    last_seq: u8,
}

fn next_seq(seq: u8) -> u8 {
    (seq + 1) & 0x0F
}

impl Model {
    fn new(select_timeout_ms: u64) -> Self {
        Self {
            select_timeout_ms,
            now_ms: 0,
            selection: None,
            last_request: None,
            handler_status: CommandStatus::Success,
            last_select_seq: 0,
            last_select_set: 0,
            last_seq: 0,
        }
    }

    fn handler_status(&self, obj: Obj) -> CommandStatus {
        if obj.index() == STATUS_INDEX {
            self.handler_status
        } else {
            CommandStatus::Success
        }
    }

    /// the handler executes every object, and the response echoes each with its status
    fn execute(&self, set: usize, seq: u8, event: impl Fn(Control) -> Event) -> (Outcome, bool) {
        let objects = SETS[set].objects;
        let mut events = vec![Event::BeginControls];
        events.extend(objects.iter().map(|obj| event(obj.control())));
        events.push(Event::EndControls);
        let response = echo(seq, set, objects.len(), |obj| {
            self.handler_status(obj).as_u8()
        });
        let success = objects
            .iter()
            .all(|obj| self.handler_status(*obj) == CommandStatus::Success);
        (
            Outcome {
                response: Some(response),
                events,
            },
            success,
        )
    }

    /// a rejected control: each object with `status`, or only the first (Rule 6, option 3)
    fn reject(set: usize, seq: u8, status: CommandStatus) -> Vec<Outcome> {
        let count = SETS[set].objects.len();
        let mut outcomes = vec![Outcome::response(echo(seq, set, count, |_| status.as_u8()))];
        if count > 1 {
            outcomes.push(Outcome::response(echo(seq, set, 1, |_| status.as_u8())));
        }
        outcomes
    }

    /// The outcomes the standard allows for `request`, each with the model that follows it
    fn step(&self, request: Request) -> Vec<(Outcome, Model)> {
        let bytes = request.bytes();
        let seq = request.seq();
        let mut next = self.clone();
        next.last_seq = seq;
        if let Request::Select(set, seq) = request {
            next.last_select_seq = seq;
            next.last_select_set = set;
        }

        match request {
            // not requests: no response, and nothing changes
            Request::Confirm { .. } | Request::NotFirst(_) => {
                return vec![(Outcome::nothing(), next)];
            }
            // TB2016-001: answered with IIN2.0, and not the last valid request. It isn't a
            // SELECT or OPERATE, so it ends the selection (Table 4-9, last row)
            Request::UnknownFunction(_) => {
                next.selection = None;
                return vec![(Outcome::response(response_header(seq, 0x01)), next)];
            }
            _ => {}
        }

        // a repeat of the last request gets its response again, byte for byte (decision 0002),
        // and only a retry of the selection's SELECT keeps the selection (Table 4-9, row 1)
        if let Some(last) = &self.last_request {
            if last.bytes == bytes {
                if self
                    .selection
                    .as_ref()
                    .is_some_and(|selection| selection.request != bytes)
                {
                    next.selection = None;
                }
                let outcome = match &last.response {
                    Some(response) => Outcome::response(response.clone()),
                    None => Outcome::nothing(),
                };
                return vec![(outcome, next)];
            }
        }

        let selected_seq = self.selection.as_ref().map(|selection| selection.seq);

        // Table 4-9, row 2: a different SELECT with the selection's sequence number is discarded
        if let Request::Select(_, seq) = request {
            if selected_seq == Some(seq) {
                return vec![(Outcome::nothing(), next)];
            }
        }

        let mut alternatives = Vec::new();

        // any other request ends the selection (Table 4-9, rows 3 and 5-7)
        let selection = next.selection.take();

        let outcomes: Vec<Outcome> = match request {
            Request::Select(set, seq) => {
                let (outcome, success) = self.execute(set, seq, Event::Select);
                // Rule 3: a non-zero status in the response cancels the selection
                if success {
                    next.selection = Some(Selection {
                        seq,
                        request: bytes.clone(),
                        set,
                        at_ms: self.now_ms,
                    });
                }
                vec![outcome]
            }
            Request::Operate(set, seq) => self.operate(selection, set, seq),
            Request::DirectOperate(set, seq) => {
                vec![
                    self.execute(set, seq, |c| Event::Operate(c, OperateType::DirectOperate))
                        .0,
                ]
            }
            Request::DirectOperateNoAck(set, seq) => {
                let (mut outcome, _) = self.execute(set, seq, |c| {
                    Event::Operate(c, OperateType::DirectOperateNoAck)
                });
                outcome.response = None;
                vec![outcome]
            }
            Request::Read(seq) => vec![Outcome::response(response_header(seq, 0))],
            // g52v2: the clock is paused, so no time passes while the outstation processes it
            Request::DelayMeasure(seq) => {
                let mut response = response_header(seq, 0);
                response.extend_from_slice(&[52, 2, 0x07, 0x01, 0x00, 0x00]);
                vec![Outcome::response(response)]
            }
            // decision 0003: answered with IIN2.2 (TB2016-001 4.5.11), even a SELECT with the
            // selection's sequence number
            Request::Malformed(_, seq) => vec![Outcome::response(response_header(seq, 0x04))],
            Request::UnknownFunction(_) | Request::Confirm { .. } | Request::NotFirst(_) => {
                panic!("{request:?} was answered above")
            }
        };

        for outcome in outcomes {
            let mut after = next.clone();
            after.last_request = Some(LastRequest {
                bytes: bytes.clone(),
                response: outcome.response.clone(),
            });
            alternatives.push((outcome, after));
        }
        alternatives
    }

    /// Rule 1, Rule 2 and Table 4-9 rows 4-6
    fn operate(&self, selection: Option<Selection>, set: usize, seq: u8) -> Vec<Outcome> {
        let Some(selection) = selection else {
            return Self::reject(set, seq, CommandStatus::NoSelect);
        };
        let elapsed = self.now_ms - selection.at_ms;
        let matches = seq == next_seq(selection.seq)
            && SETS[set].request_objects() == SETS[selection.set].request_objects();
        let executed = || {
            self.execute(set, seq, |c| {
                Event::Operate(c, OperateType::SelectBeforeOperate)
            })
            .0
        };

        if matches {
            match elapsed.cmp(&self.select_timeout_ms) {
                std::cmp::Ordering::Less => vec![executed()],
                std::cmp::Ordering::Equal => {
                    let mut outcomes = vec![executed()];
                    outcomes.extend(Self::reject(set, seq, CommandStatus::Timeout));
                    outcomes
                }
                std::cmp::Ordering::Greater => Self::reject(set, seq, CommandStatus::Timeout),
            }
        } else {
            let mut outcomes = Self::reject(set, seq, CommandStatus::NoSelect);
            if elapsed > self.select_timeout_ms {
                outcomes.extend(Self::reject(set, seq, CommandStatus::Timeout));
            }
            outcomes
        }
    }

    fn advance(&mut self, advance: Advance) -> u64 {
        let t = self.select_timeout_ms;
        let ms = match advance {
            Advance::BeforeTimeout => t - 1,
            Advance::AtTimeout => t,
            Advance::AfterTimeout => t + 1,
            Advance::Millis(ms) => ms,
        };
        self.now_ms += ms;
        ms
    }

    fn resolve_seq(&self, seq: SeqChoice) -> u8 {
        let selected = self
            .selection
            .as_ref()
            .map_or(self.last_select_seq, |selection| selection.seq);
        match seq {
            SeqChoice::Selected => selected,
            SeqChoice::AfterSelected => next_seq(selected),
            SeqChoice::Next => next_seq(self.last_seq),
            SeqChoice::Fixed(seq) => seq,
        }
    }

    fn resolve_set(&self, set: SetChoice) -> usize {
        match set {
            SetChoice::Selected => self
                .selection
                .as_ref()
                .map_or(self.last_select_set, |selection| selection.set),
            SetChoice::Fixed(set) => set,
        }
    }

    fn resolve(&self, op: RequestOp) -> Request {
        match op {
            RequestOp::Select(set, seq) => {
                Request::Select(self.resolve_set(set), self.resolve_seq(seq))
            }
            RequestOp::Operate(set, seq) => {
                Request::Operate(self.resolve_set(set), self.resolve_seq(seq))
            }
            RequestOp::DirectOperate(set, seq) => {
                Request::DirectOperate(self.resolve_set(set), self.resolve_seq(seq))
            }
            RequestOp::DirectOperateNoAck(set, seq) => {
                Request::DirectOperateNoAck(self.resolve_set(set), self.resolve_seq(seq))
            }
            RequestOp::Read(seq) => Request::Read(self.resolve_seq(seq)),
            RequestOp::DelayMeasure(seq) => Request::DelayMeasure(self.resolve_seq(seq)),
            RequestOp::Malformed(function, seq) => {
                Request::Malformed(function, self.resolve_seq(seq))
            }
            RequestOp::UnknownFunction(seq) => Request::UnknownFunction(self.resolve_seq(seq)),
            RequestOp::Confirm(uns, seq) => Request::Confirm {
                uns,
                seq: self.resolve_seq(seq),
            },
            RequestOp::NotFirst(seq) => Request::NotFirst(self.resolve_seq(seq)),
        }
    }
}

/// a sequence number relative to the model's state, so that shrinking keeps its meaning
#[derive(Copy, Clone, Debug)]
enum SeqChoice {
    /// the selection's, or the last SELECT's
    Selected,
    /// one after [`SeqChoice::Selected`], wrapping from 15 to 0
    AfterSelected,
    /// one after the last request's
    Next,
    Fixed(u8),
}

#[derive(Copy, Clone, Debug)]
enum SetChoice {
    /// the selection's objects, or the last SELECT's
    Selected,
    Fixed(usize),
}

#[derive(Copy, Clone, Debug)]
enum RequestOp {
    Select(SetChoice, SeqChoice),
    Operate(SetChoice, SeqChoice),
    DirectOperate(SetChoice, SeqChoice),
    DirectOperateNoAck(SetChoice, SeqChoice),
    Read(SeqChoice),
    DelayMeasure(SeqChoice),
    Malformed(FunctionCode, SeqChoice),
    UnknownFunction(SeqChoice),
    Confirm(bool, SeqChoice),
    NotFirst(SeqChoice),
}

#[derive(Copy, Clone, Debug)]
enum Advance {
    /// one millisecond before the select timeout
    BeforeTimeout,
    AtTimeout,
    AfterTimeout,
    Millis(u64),
}

#[derive(Copy, Clone, Debug)]
enum Op {
    Request(RequestOp),
    /// send the last request's bytes again
    Repeat,
    Advance(Advance),
    /// the status the control handler returns for [`STATUS_INDEX`] from now on
    SetHandlerStatus(CommandStatus),
    /// a SELECT, then `Between`, then the OPERATE that matches the SELECT, which shows whether
    /// `Between` ended the selection or kept it
    SelectThenOperate(SetChoice, Between),
}

/// what [`Op::SelectThenOperate`] puts between its SELECT and OPERATE
#[derive(Copy, Clone, Debug)]
enum Between {
    Request(RequestOp),
    /// the SELECT again
    Repeat,
    Advance(Advance),
}

/// sequence numbers weighted towards `likely`
fn seq_choice(likely: SeqChoice) -> impl Strategy<Value = SeqChoice> {
    prop_oneof![
        6 => Just(likely),
        1 => Just(SeqChoice::Selected),
        1 => Just(SeqChoice::AfterSelected),
        1 => Just(SeqChoice::Next),
        // the wrap from 15 to 0
        1 => Just(SeqChoice::Fixed(15)),
        1 => (0u8..16).prop_map(SeqChoice::Fixed),
    ]
}

/// object sets weighted towards the selection's
fn set_choice() -> impl Strategy<Value = SetChoice> {
    prop_oneof![
        4 => Just(SetChoice::Selected),
        1 => Just(SetChoice::Fixed(0)),
        2 => (0..SETS.len()).prop_map(SetChoice::Fixed),
    ]
}

fn request_op() -> impl Strategy<Value = RequestOp> {
    let next = || seq_choice(SeqChoice::Next);
    prop_oneof![
        6 => (set_choice(), next()).prop_map(|(set, seq)| RequestOp::Select(set, seq)),
        2 => (set_choice(), seq_choice(SeqChoice::Selected))
            .prop_map(|(set, seq)| RequestOp::Select(set, seq)),
        // a SELECT whose object at the index with the changing handler status is first or last
        2 => (prop_oneof![Just(FAILING_FIRST), Just(FAILING_LAST)], next())
            .prop_map(|(set, seq)| RequestOp::Select(SetChoice::Fixed(set), seq)),
        // the OPERATE that matches the selection
        8 => Just(RequestOp::Operate(SetChoice::Selected, SeqChoice::AfterSelected)),
        4 => (set_choice(), seq_choice(SeqChoice::AfterSelected))
            .prop_map(|(set, seq)| RequestOp::Operate(set, seq)),
        2 => (set_choice(), next()).prop_map(|(set, seq)| RequestOp::DirectOperate(set, seq)),
        1 => (set_choice(), next())
            .prop_map(|(set, seq)| RequestOp::DirectOperateNoAck(set, seq)),
        2 => next().prop_map(RequestOp::Read),
        1 => next().prop_map(RequestOp::DelayMeasure),
        2 => (
            prop_oneof![
                Just(FunctionCode::Select),
                Just(FunctionCode::Operate),
                Just(FunctionCode::Read)
            ],
            seq_choice(SeqChoice::Selected)
        )
            .prop_map(|(function, seq)| RequestOp::Malformed(function, seq)),
        1 => next().prop_map(RequestOp::UnknownFunction),
        1 => (any::<bool>(), seq_choice(SeqChoice::Selected))
            .prop_map(|(uns, seq)| RequestOp::Confirm(uns, seq)),
        1 => next().prop_map(RequestOp::NotFirst),
    ]
}

fn advance() -> impl Strategy<Value = Advance> {
    prop_oneof![
        2 => Just(Advance::BeforeTimeout),
        2 => Just(Advance::AtTimeout),
        2 => Just(Advance::AfterTimeout),
        1 => Just(Advance::Millis(1)),
        1 => (0u64..3000).prop_map(Advance::Millis),
    ]
}

/// each request or advance that keeps or ends a selection about equally often
fn between() -> impl Strategy<Value = Between> {
    let request = |op| Just(Between::Request(op));
    let any_set = || (0..SETS.len()).prop_map(SetChoice::Fixed);
    let malformed = prop_oneof![
        Just(FunctionCode::Select),
        Just(FunctionCode::Operate),
        Just(FunctionCode::Read)
    ];
    prop_oneof![
        // keep it
        1 => request(RequestOp::Confirm(false, SeqChoice::Selected)),
        1 => request(RequestOp::Confirm(true, SeqChoice::Selected)),
        1 => request(RequestOp::NotFirst(SeqChoice::Next)),
        1 => Just(Between::Repeat),
        // discarded, or a retry if the objects are the same
        1 => any_set()
            .prop_map(|set| Between::Request(RequestOp::Select(set, SeqChoice::Selected))),
        2 => advance().prop_map(Between::Advance),
        // end it
        1 => any_set().prop_map(|set| Between::Request(RequestOp::Select(set, SeqChoice::Next))),
        1 => (set_choice(), seq_choice(SeqChoice::AfterSelected))
            .prop_map(|(set, seq)| Between::Request(RequestOp::Operate(set, seq))),
        1 => set_choice()
            .prop_map(|set| Between::Request(RequestOp::DirectOperate(set, SeqChoice::Next))),
        1 => set_choice().prop_map(|set| {
            Between::Request(RequestOp::DirectOperateNoAck(set, SeqChoice::Next))
        }),
        1 => request(RequestOp::Read(SeqChoice::Next)),
        1 => request(RequestOp::DelayMeasure(SeqChoice::Next)),
        1 => request(RequestOp::UnknownFunction(SeqChoice::Next)),
        3 => (malformed, prop_oneof![Just(SeqChoice::Selected), Just(SeqChoice::Next)])
            .prop_map(|(function, seq)| Between::Request(RequestOp::Malformed(function, seq))),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        24 => request_op().prop_map(Op::Request),
        4 => Just(Op::Repeat),
        4 => advance().prop_map(Op::Advance),
        2 => prop_oneof![
            Just(CommandStatus::Success),
            Just(CommandStatus::HardwareError),
            Just(CommandStatus::NotAuthorized),
        ]
        .prop_map(Op::SetHandlerStatus),
        6 => (set_choice(), between())
            .prop_map(|(set, between)| Op::SelectThenOperate(set, between)),
    ]
}

/// the shortest select timeout, a few short ones, and the default
fn select_timeout_ms() -> impl Strategy<Value = u64> {
    prop_oneof![Just(1u64), Just(2), Just(50), Just(1000), Just(5000),]
}

/// Run a test on a current-thread runtime with the clock paused, under a [`watchdog`]
fn run_paused(test: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(watchdog(test))
}

fn config(select_timeout_ms: u64) -> OutstationConfig {
    let mut config = get_default_config();
    config.keep_alive_timeout = None;
    config.select_timeout = Timeout::from_millis(select_timeout_ms).unwrap();
    config
}

/// the outstation under test and the model it is checked against
struct Run {
    harness: OutstationHarness,
    start: Instant,
    model: Model,
    /// the last request sent
    last: Option<Request>,
    /// the select-before-operate controls the handler has operated
    operations: usize,
}

impl Run {
    fn new(select_timeout_ms: u64) -> Self {
        Self {
            harness: new_harness(config(select_timeout_ms)),
            start: Instant::now(),
            model: Model::new(select_timeout_ms),
            last: None,
            operations: 0,
        }
    }

    async fn apply(&mut self, op: Op) {
        match op {
            Op::Request(op) => self.send(self.model.resolve(op)).await,
            Op::Repeat => self.repeat().await,
            Op::Advance(advance) => self.advance(advance).await,
            Op::SetHandlerStatus(status) => {
                self.harness.set_control_status(STATUS_INDEX, status);
                self.model.handler_status = status;
            }
            Op::SelectThenOperate(set, between) => {
                let set = self.model.resolve_set(set);
                let seq = self.model.resolve_seq(SeqChoice::Next);
                self.send(Request::Select(set, seq)).await;
                match between {
                    Between::Request(op) => self.send(self.model.resolve(op)).await,
                    Between::Repeat => self.repeat().await,
                    Between::Advance(advance) => self.advance(advance).await,
                }
                self.send(Request::Operate(set, next_seq(seq))).await;
            }
        }
    }

    async fn repeat(&mut self) {
        if let Some(request) = self.last {
            self.send(request).await;
        }
    }

    async fn advance(&mut self, advance: Advance) {
        let ms = self.model.advance(advance);
        tokio::time::advance(Duration::from_millis(ms)).await;
        // no timer is running: nothing is sent and the clock moved by exactly `ms`
        self.harness.expect_no_response();
        self.harness.check_no_events();
        assert_eq!(
            self.start.elapsed(),
            Duration::from_millis(self.model.now_ms)
        );
    }

    async fn send(&mut self, request: Request) {
        self.last = Some(request);

        // the outstation has answered once this returns (see `pop_response`)
        self.harness.send_and_process(&request.bytes()).await;
        let mut events = Vec::new();
        while let Some(event) = self.harness.poll_event() {
            events.push(event);
        }
        let mut responses = Vec::new();
        while let Some(response) = self.harness.pop_response() {
            responses.push(response);
        }
        assert!(
            responses.len() <= 1,
            "{request:?} at {} ms: more than one response: {responses:02X?}",
            self.model.now_ms
        );
        let actual = Outcome {
            response: responses.pop(),
            events,
        };
        self.operations += actual
            .events
            .iter()
            .filter(|event| matches!(event, Event::Operate(_, OperateType::SelectBeforeOperate)))
            .count();

        let alternatives = self.model.step(request);
        match alternatives
            .iter()
            .position(|(outcome, _)| *outcome == actual)
        {
            Some(position) => self.model = alternatives[position].1.clone(),
            None => {
                let expected: Vec<&Outcome> =
                    alternatives.iter().map(|(outcome, _)| outcome).collect();
                panic!(
                    "{request:?} at {} ms: got {actual:?}, expected one of {expected:?}",
                    self.model.now_ms
                )
            }
        }
    }

    /// nothing follows the last request
    async fn finish(mut self) {
        tokio::task::yield_now().await;
        self.harness.expect_no_response();
        self.harness.check_no_events();
    }
}

async fn check_model(select_timeout_ms: u64, ops: Vec<Op>) {
    let mut run = Run::new(select_timeout_ms);
    for op in ops {
        run.apply(op).await;
    }
    run.finish().await;
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn select_and_operate_follow_table_4_9(
        select_timeout_ms in select_timeout_ms(),
        ops in prop::collection::vec(op(), 1..32),
    ) {
        run_paused(check_model(select_timeout_ms, ops));
    }
}

/// Rule 3: a SELECT that fails ends the selection it replaces, which the generator rarely
/// reaches because the handler has to fail first
#[test]
fn failed_select_ends_the_previous_selection() {
    let ops = vec![
        Op::SetHandlerStatus(CommandStatus::HardwareError),
        Op::SelectThenOperate(
            SetChoice::Fixed(0),
            Between::Request(RequestOp::Select(
                SetChoice::Fixed(FAILING_FIRST),
                SeqChoice::Next,
            )),
        ),
    ];
    run_paused(check_model(1000, ops));
}

/// the select timeout of the op lists below, which don't advance the clock
const SELECT_TIMEOUT_MS: u64 = 1000;

/// a SELECT of the first set with sequence number 0
const SELECT: Op = Op::Request(RequestOp::Select(SetChoice::Fixed(0), SeqChoice::Fixed(0)));

/// the OPERATE that matches [`SELECT`]
const OPERATE: Op = Op::Request(RequestOp::Operate(SetChoice::Fixed(0), SeqChoice::Fixed(1)));

/// Runs `ops` against the model, and checks that the handler operates `operations` controls
/// select-before-operate, the count each op list expects whatever the model predicts
fn check_operations(ops: &[Op], operations: usize) {
    run_paused(async {
        let mut run = Run::new(SELECT_TIMEOUT_MS);
        for op in ops {
            run.apply(*op).await;
        }
        assert_eq!(run.operations, operations, "{ops:?}");
        run.finish().await;
    });
}

/// [`SELECT`], `between`, then [`OPERATE`]
fn select_then_operate(between: &[Op]) -> Vec<Op> {
    [&[SELECT], between, &[OPERATE]].concat()
}

/// A solicited or unsolicited CONFIRM and a fragment with FIR = 0 aren't requests (Rule 4), and
/// a SELECT of other objects with the selection's sequence number is discarded (Table 4-9, row
/// 2): each keeps the selection
#[test]
fn confirms_invalid_fragments_and_discarded_selects_keep_the_selection() {
    for between in [
        RequestOp::Confirm(false, SeqChoice::Fixed(0)),
        RequestOp::Confirm(true, SeqChoice::Fixed(0)),
        RequestOp::NotFirst(SeqChoice::Fixed(0)),
        // index 8 instead of 7
        RequestOp::Select(SetChoice::Fixed(2), SeqChoice::Fixed(0)),
    ] {
        check_operations(&select_then_operate(&[Op::Request(between)]), 1);
    }
}

/// Any other request ends the selection (Rule 4, Table 4-9), the retry of a DIRECT_OPERATE
/// doesn't restore it, and neither does a SELECT that replaced it
#[test]
fn other_requests_end_the_selection() {
    let direct_operate = Op::Request(RequestOp::DirectOperate(
        SetChoice::Fixed(INDEX_0),
        SeqChoice::Fixed(5),
    ));
    for between in [
        RequestOp::Read(SeqChoice::Fixed(5)),
        RequestOp::DelayMeasure(SeqChoice::Fixed(5)),
        RequestOp::Malformed(FunctionCode::Read, SeqChoice::Fixed(5)),
        // decision 0003: a new request, not the discarded SELECT of Table 4-9 row 2
        RequestOp::Malformed(FunctionCode::Select, SeqChoice::Fixed(0)),
        RequestOp::UnknownFunction(SeqChoice::Fixed(5)),
        RequestOp::DirectOperateNoAck(SetChoice::Fixed(INDEX_0), SeqChoice::Fixed(5)),
        // rejected, at index 8 instead of 7
        RequestOp::Operate(SetChoice::Fixed(2), SeqChoice::Fixed(1)),
        RequestOp::Select(SetChoice::Fixed(INDEX_0), SeqChoice::Fixed(5)),
    ] {
        check_operations(&select_then_operate(&[Op::Request(between)]), 0);
    }
    check_operations(&select_then_operate(&[direct_operate, Op::Repeat]), 0);
}

/// Each selection operates once, by the OPERATE that matches the latest SELECT
#[test]
fn a_selection_operates_once_for_the_latest_select() {
    let request = |op| Op::Request(op);
    let cases = [
        // the retry of the OPERATE repeats its response without operating again
        (vec![SELECT, OPERATE, Op::Repeat], 1),
        (vec![OPERATE], 0),
        (
            vec![
                SELECT,
                request(RequestOp::Select(
                    SetChoice::Fixed(INDEX_0),
                    SeqChoice::Fixed(5),
                )),
                request(RequestOp::Operate(
                    SetChoice::Fixed(INDEX_0),
                    SeqChoice::Fixed(6),
                )),
            ],
            1,
        ),
        (
            vec![
                request(RequestOp::DirectOperate(
                    SetChoice::Fixed(INDEX_0),
                    SeqChoice::Fixed(4),
                )),
                request(RequestOp::DirectOperateNoAck(
                    SetChoice::Fixed(INDEX_0),
                    SeqChoice::Fixed(5),
                )),
                SELECT,
                OPERATE,
            ],
            1,
        ),
        // no longer a retry, since the OPERATE came between them
        (vec![SELECT, OPERATE, SELECT, OPERATE], 2),
    ];
    for (ops, operations) in cases {
        check_operations(&ops, operations);
    }
}

/// The model rejects an OPERATE that executes after a READ ended its selection
#[test]
fn model_rejects_an_operate_after_a_read() {
    let model = Model::new(1000);
    let (_, model) = model.step(Request::Select(0, 0)).remove(0);
    let (_, model) = model.step(Request::Read(1)).remove(0);
    let alternatives = model.step(Request::Operate(0, 1));

    let executed = Outcome {
        response: Some(echo(1, 0, 1, |_| 0)),
        events: operated(OperateType::SelectBeforeOperate).to_vec(),
    };
    let rejected = Outcome::response(echo(1, 0, 1, |_| CommandStatus::NoSelect.as_u8()));
    let outcomes: Vec<Outcome> = alternatives
        .into_iter()
        .map(|(outcome, _)| outcome)
        .collect();
    assert_eq!(outcomes, vec![rejected]);
    assert!(!outcomes.contains(&executed));
}

/// The model only accepts an OPERATE of the selection one millisecond before its timeout if it
/// executes, and only rejects it with TIMEOUT one millisecond after
#[test]
fn model_decides_the_select_timeout_off_the_boundary() {
    let executed = Outcome {
        response: Some(echo(1, 0, 1, |_| 0)),
        events: operated(OperateType::SelectBeforeOperate).to_vec(),
    };
    let timed_out = Outcome::response(echo(1, 0, 1, |_| CommandStatus::Timeout.as_u8()));

    let outcomes = |advance| {
        let model = Model::new(1000);
        let (_, mut model) = model.step(Request::Select(0, 0)).remove(0);
        model.advance(advance);
        model
            .step(Request::Operate(0, 1))
            .into_iter()
            .map(|(outcome, _)| outcome)
            .collect::<Vec<_>>()
    };

    assert_eq!(outcomes(Advance::BeforeTimeout), vec![executed.clone()]);
    assert_eq!(
        outcomes(Advance::AtTimeout),
        vec![executed, timed_out.clone()]
    );
    assert_eq!(outcomes(Advance::AfterTimeout), vec![timed_out]);
}
