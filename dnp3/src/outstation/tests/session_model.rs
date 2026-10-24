// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The outstation session against a model of its event buffer, IIN, sequence numbers and confirm
//! waits
//!
//! The model keeps every buffered event by id and state, the solicited and unsolicited confirm
//! waits with their deadlines, the unsolicited retry delay, the enabled classes and the restart IIN.
//! From those it predicts every response the outstation sends after each input, and every
//! information and confirm callback, in order:
//! - an update creates an event with the next id, evicting the oldest event of its type, whatever
//!   its state, when the type is full;
//! - a response with events carries a non-empty prefix, oldest first, of the events selected for
//!   it: by the READ's headers, or every event of the enabled classes. The outstation chooses how
//!   many fit. A READ response has FIN once it carries the last of them, and CON if it carries
//!   events or isn't the last;
//! - IIN1.1-1.3 report the events that no response has carried since they were last returned to
//!   the buffer (IEEE 1815-2012 4.5.2-4.5.4), IIN1.7 stays set until the master clears it, and
//!   IIN2.3 is set by an eviction and cleared by a confirm that leaves room in every type (4.5.12);
//! - only the confirm of a response, with its sequence number and UNS bit, before its timeout,
//!   deletes the response's events, each reported by `Cleared`, followed by the buffer state that's
//!   left. A timeout, a new request or a lost connection returns them to the buffer (Table 6-1);
//! - an unsolicited response is sent again byte for byte on timeout, up to the retry limit, and a
//!   new series waits for the retry delay. The null response is sent again with a new sequence
//!   number until it's confirmed. A DISABLE_UNSOLICITED during the wait ends it on timeout without
//!   a retry (TB2015-002a, decision 0007 in the protocol reference);
//! - a repeated request other than a READ gets the stored response's bytes (decision 0002), and a
//!   repeated READ in the solicited confirm wait gets the current fragment's bytes and restarts the
//!   wait. Elsewhere a repeated READ is answered as a new one, a deliberate deviation. A request
//!   with the sequence number of the last one but other contents is a new request (Table 6-1 row
//!   18);
//! - a READ during the unsolicited confirm wait gets no response until the wait ends (Table 6-1 row
//!   25, TB2015-002a rule 16). A later READ replaces it and any other request discards it. While
//!   one is held, a timeout ends the wait without a retry. Once the wait ends, by its confirm or a
//!   timeout, the READ is answered from the events left, before the next unsolicited response, and
//!   only then becomes the last request a repeat matches.
//!
//! The outstation has handled each input when the test looks at what it did, on the guarantee
//! that `pop_response` describes: the outstation has answered a request once the test sees it read,
//! and an input that only wakes the outstation (its start, an update, a timer, a new connection)
//! is handled in the outstation's next poll, which a yield lets run first. Time only moves when the
//! test sleeps, which lets the paused clock fire the outstation's timers on the way, in order.
//! After each input the test takes every response and callback, so the model accepts no more than
//! it expects, and checks that the clock agrees with the model's.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the model adds small sequence numbers, ids, counts and millisecond times"
)]

use std::collections::VecDeque;
use std::ops::Range;

use proptest::prelude::*;
use tokio::time::{Duration, Instant};

use crate::app::measurement::{AnalogInput, Counter, Flags, Time};
use crate::app::{BufferSize, Timeout};
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::{
    Add, AnalogInputConfig, CounterConfig, Database, EventBufferConfig, EventClass, Update,
    UpdateInfo, UpdateOptions,
};
use crate::outstation::tests::harness::*;
use crate::outstation::{BufferState, ClassCount, TypeCount};
use crate::util::test_support::run_paused;

/// points of each kind, at indices 0 to 3
const POINTS: u16 = 4;

/// the flags of every event
const ONLINE: u8 = 0x01;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Kind {
    Counter,
    Analog,
}

impl Kind {
    /// the class of the point at `index`, 0 for class 1
    fn class(self, index: u16) -> u8 {
        let classes = match self {
            Kind::Counter => [0, 1, 2, 0],
            Kind::Analog => [1, 2, 0, 2],
        };
        classes[usize::from(index)]
    }

    /// the group of its events, read as variation 0 and reported as variation 1
    fn group(self) -> u8 {
        match self {
            Kind::Counter => 22,
            Kind::Analog => 32,
        }
    }
}

fn event_class(class: u8) -> EventClass {
    [EventClass::Class1, EventClass::Class2, EventClass::Class3][usize::from(class)]
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum EventState {
    Unselected,
    /// selected for the response being built
    Selected,
    /// in a response that hasn't been confirmed
    Written,
}

/// an event in the buffer, whose value is its id plus one
#[derive(Copy, Clone, Debug)]
struct Buffered {
    id: u64,
    kind: Kind,
    index: u16,
    class: u8,
    state: EventState,
}

/// an event object in a response
#[derive(Copy, Clone, Debug, PartialEq)]
struct EventObject {
    kind: Kind,
    index: u16,
    flags: u8,
    value: u64,
}

/// Decode the objects of a response that carries only g22v1 and g32v1 events, with 2-byte counts
/// and index prefixes
fn decode_events(mut bytes: &[u8]) -> Result<Vec<EventObject>, String> {
    let mut objects = Vec::new();
    while !bytes.is_empty() {
        let [group, variation, qualifier, count_low, count_high, rest @ ..] = bytes else {
            return Err(format!("the header is cut off: {bytes:02X?}"));
        };
        let kind = match (*group, *variation) {
            (22, 1) => Kind::Counter,
            (32, 1) => Kind::Analog,
            _ => return Err(format!("unexpected object g{group}v{variation}")),
        };
        if *qualifier != 0x28 {
            return Err(format!("unexpected qualifier {qualifier:#04X}"));
        }
        let count = u16::from_le_bytes([*count_low, *count_high]);
        if count == 0 {
            return Err("a header has no objects".to_string());
        }
        bytes = rest;
        for _ in 0..count {
            let [i0, i1, flags, v0, v1, v2, v3, rest @ ..] = bytes else {
                return Err(format!("the object is cut off: {bytes:02X?}"));
            };
            let raw = [*v0, *v1, *v2, *v3];
            let value = match kind {
                Kind::Counter => u64::from(u32::from_le_bytes(raw)),
                Kind::Analog => u64::try_from(i32::from_le_bytes(raw))
                    .map_err(|_| format!("negative analog value: {raw:02X?}"))?,
            };
            objects.push(EventObject {
                kind,
                index: u16::from_le_bytes([*i0, *i1]),
                flags: *flags,
                value,
            });
            bytes = rest;
        }
    }
    Ok(objects)
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ReadHeader {
    /// g60v2-v4 for the class, all events or a limited count
    Class(u8, Option<u8>),
    /// g22v0 or g32v0, all events or a limited count
    Type(Kind, Option<u8>),
}

impl ReadHeader {
    fn limit(self) -> Option<u8> {
        match self {
            ReadHeader::Class(_, limit) | ReadHeader::Type(_, limit) => limit,
        }
    }

    fn matches(self, event: &Buffered) -> bool {
        match self {
            ReadHeader::Class(class, _) => event.class == class,
            ReadHeader::Type(kind, _) => event.kind == kind,
        }
    }

    fn write(self, out: &mut Vec<u8>) {
        match self {
            ReadHeader::Class(class, _) => out.extend([60, class + 2]),
            ReadHeader::Type(kind, _) => out.extend([kind.group(), 0]),
        }
        match self.limit() {
            None => out.push(0x06),
            Some(count) => out.extend([0x07, count]),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Function {
    Read(Vec<ReadHeader>),
    DelayMeasure,
    /// WRITE of IIN1.7 to 0
    ClearRestart,
    EnableUnsolicited([bool; 3]),
    DisableUnsolicited([bool; 3]),
}

#[derive(Clone, Debug, PartialEq)]
struct Request {
    seq: u8,
    function: Function,
}

impl Request {
    fn bytes(&self) -> Vec<u8> {
        let control = 0xC0 | self.seq;
        match &self.function {
            Function::Read(headers) => {
                let mut bytes = vec![control, 0x01];
                for header in headers {
                    header.write(&mut bytes);
                }
                bytes
            }
            Function::DelayMeasure => delay_measure(self.seq).to_vec(),
            Function::ClearRestart => vec![control, 0x02, 80, 1, 0x00, 7, 7, 0x00],
            Function::EnableUnsolicited(named) => {
                let mut bytes = vec![control, 0x14];
                for (class, _) in (0u8..).zip(named).filter(|(_, named)| **named) {
                    bytes.extend([60, class + 2, 0x06]);
                }
                bytes
            }
            Function::DisableUnsolicited(named) => {
                super::harness::disable_unsolicited(self.seq, *named)
            }
        }
    }
}

/// the configuration of a case
#[derive(Copy, Clone, Debug)]
struct Setup {
    unsolicited: bool,
    max_retries: Option<usize>,
    confirm_ms: u64,
    retry_ms: u64,
    max_counter: u16,
    max_analog: u16,
    solicited_size: usize,
    unsolicited_size: usize,
}

#[derive(Copy, Clone, Debug)]
enum Wait {
    Idle,
    /// the confirm of the fragment with `ecsn`, the last one in its series if `fin`
    Solicited {
        ecsn: u8,
        fin: bool,
        deadline: u64,
    },
    Unsolicited {
        seq: u8,
        null: bool,
        /// the retries left, `None` for no limit
        retries: Option<usize>,
        deadline: u64,
        /// a DISABLE_UNSOLICITED was processed during the wait
        disabled: bool,
    },
}

#[derive(Copy, Clone, Debug)]
enum UnsolicitedState {
    NullRequired,
    /// ready to send, from the end of the retry delay if there is one
    Ready(Option<u64>),
}

#[derive(Clone, Debug)]
enum Input {
    /// the session starts
    Start,
    /// events for a point, one per id
    Update {
        kind: Kind,
        index: u16,
        ids: Range<u64>,
    },
    Request(Request),
    Confirm {
        uns: bool,
        seq: u8,
    },
    Advance(u64),
    /// the connection drops and a new one starts
    Reconnect,
}

/// what the model predicts for an input, besides the responses
struct Outcome {
    callbacks: Vec<Event>,
    updates: Vec<UpdateInfo>,
}

struct Model {
    setup: Setup,
    now: u64,
    events: Vec<Buffered>,
    next_id: u64,
    overflow: bool,
    restart: bool,
    classes: [bool; 3],
    unsolicited: UnsolicitedState,
    next_unsolicited_seq: u8,
    wait: Wait,
    next_request_seq: u8,
    /// the request the master sent last
    last_request: Option<Request>,
    /// the request the outstation answered last, which a repeat matches
    last_valid: Option<Request>,
    /// the solicited response sent last
    last_solicited: Option<Vec<u8>>,
    /// the READ received during the unsolicited confirm wait, answered when the wait ends
    deferred: Option<(u8, Vec<ReadHeader>)>,
    /// the unsolicited response sent last
    last_unsolicited: Option<Vec<u8>>,
    /// the responses that the input made, in the order sent
    writes: VecDeque<Vec<u8>>,
    callbacks: Vec<Event>,
    /// the input being checked, for failure messages
    context: String,
}

fn next_seq(seq: u8) -> u8 {
    (seq + 1) % 16
}

fn previous_seq(seq: u8) -> u8 {
    (seq + 15) % 16
}

impl Model {
    fn new(setup: Setup) -> Self {
        Self {
            setup,
            now: 0,
            events: Vec::new(),
            next_id: 0,
            overflow: false,
            restart: true,
            classes: [false; 3],
            unsolicited: UnsolicitedState::NullRequired,
            next_unsolicited_seq: 0,
            wait: Wait::Idle,
            next_request_seq: 0,
            last_request: None,
            last_valid: None,
            last_solicited: None,
            deferred: None,
            last_unsolicited: None,
            writes: VecDeque::new(),
            callbacks: Vec::new(),
            context: String::new(),
        }
    }

    // ---- inputs ----

    /// The input for an op in the current state, or `None` if the op doesn't apply
    fn resolve(&mut self, op: &Op) -> Option<Input> {
        match op {
            Op::Update { kind, index, count } => Some(Input::Update {
                kind: *kind,
                index: *index,
                ids: self.next_id..self.next_id + count,
            }),
            Op::Request(function) => {
                let seq = self.next_request_seq;
                self.next_request_seq = next_seq(seq);
                Some(Input::Request(Request {
                    seq,
                    function: function.clone(),
                }))
            }
            Op::SameSeq(function) => Some(Input::Request(Request {
                seq: self.last_request.as_ref()?.seq,
                function: function.clone(),
            })),
            Op::Repeat => self.last_request.clone().map(Input::Request),
            Op::SolicitedConfirm(choice) => {
                let base = match self.wait {
                    Wait::Solicited { ecsn, .. } => ecsn,
                    _ => previous_seq(self.next_request_seq),
                };
                Some(Input::Confirm {
                    uns: *choice == ConfirmChoice::OtherKind,
                    seq: choice.seq(base),
                })
            }
            Op::UnsolicitedConfirm(choice) => {
                let base = match self.wait {
                    Wait::Unsolicited { seq, .. } => seq,
                    _ => previous_seq(self.next_unsolicited_seq),
                };
                Some(Input::Confirm {
                    uns: *choice != ConfirmChoice::OtherKind,
                    seq: choice.seq(base),
                })
            }
            Op::Advance(choice) => Some(Input::Advance(self.advance_ms(*choice))),
            Op::Reconnect => Some(Input::Reconnect),
        }
    }

    fn next_deadline(&self) -> Option<u64> {
        match self.wait {
            Wait::Solicited { deadline, .. } | Wait::Unsolicited { deadline, .. } => Some(deadline),
            Wait::Idle => match self.unsolicited {
                UnsolicitedState::Ready(Some(retry_at))
                    if self.setup.unsolicited && retry_at > self.now =>
                {
                    Some(retry_at)
                }
                _ => None,
            },
        }
    }

    fn advance_ms(&self, choice: AdvanceChoice) -> u64 {
        let until = self.next_deadline().map(|deadline| deadline - self.now);
        match (choice, until) {
            (AdvanceChoice::BeforeDeadline, Some(ms)) => ms - 1,
            (AdvanceChoice::AtDeadline, Some(ms)) => ms,
            (AdvanceChoice::AfterDeadline, Some(ms)) => ms + 1,
            (AdvanceChoice::Millis(ms), _) => ms,
            (_, None) => self.setup.confirm_ms,
        }
    }

    /// The input that moves the final drain on: confirm whatever is awaited, disable unsolicited
    /// responses, then READ every class until the buffer is empty
    fn drain_op(&self) -> Option<Op> {
        Some(match self.wait {
            Wait::Unsolicited { .. } => Op::UnsolicitedConfirm(ConfirmChoice::Expected),
            Wait::Solicited { .. } => Op::SolicitedConfirm(ConfirmChoice::Expected),
            Wait::Idle if self.setup.unsolicited && self.classes.contains(&true) => {
                Op::Request(Function::DisableUnsolicited([true; 3]))
            }
            Wait::Idle if self.events.is_empty() => return None,
            Wait::Idle => Op::Request(read_every_class()),
        })
    }

    /// Apply an input to the model, checking the responses the outstation sent for it
    fn apply(&mut self, input: &Input, writes: Vec<Vec<u8>>) -> Outcome {
        self.context = format!("{input:?} at {} ms", self.now);
        self.writes = writes.into();
        self.callbacks.clear();
        let mut updates = Vec::new();

        match input {
            Input::Start => self.run_idle(),
            Input::Update { kind, index, ids } => {
                updates = self.update(*kind, *index, ids.clone());
                self.run_idle();
            }
            Input::Request(request) => {
                self.last_request = Some(request.clone());
                self.request(request.clone());
            }
            Input::Confirm { uns, seq } => self.confirm(*uns, *seq),
            Input::Advance(ms) => self.advance(*ms),
            Input::Reconnect => self.reconnect(),
        }

        assert!(
            self.writes.is_empty(),
            "{}: unexpected responses {:02X?}",
            self.context,
            self.writes
        );
        Outcome {
            callbacks: std::mem::take(&mut self.callbacks),
            updates,
        }
    }

    // ---- the event buffer ----

    fn type_count(&self, kind: Kind) -> usize {
        self.events
            .iter()
            .filter(|event| event.kind == kind)
            .count()
    }

    fn max(&self, kind: Kind) -> usize {
        usize::from(match kind {
            Kind::Counter => self.setup.max_counter,
            Kind::Analog => self.setup.max_analog,
        })
    }

    fn is_any_full(&self) -> bool {
        [Kind::Counter, Kind::Analog]
            .into_iter()
            .any(|kind| self.type_count(kind) >= self.max(kind))
    }

    fn update(&mut self, kind: Kind, index: u16, ids: Range<u64>) -> Vec<UpdateInfo> {
        let mut updates = Vec::new();
        for id in ids {
            assert_eq!(id, self.next_id);
            self.next_id += 1;
            let discarded = if self.type_count(kind) >= self.max(kind) {
                let oldest = self.events.iter().position(|event| event.kind == kind);
                oldest.map(|position| self.events.remove(position).id)
            } else {
                None
            };
            self.events.push(Buffered {
                id,
                kind,
                index,
                class: kind.class(index),
                state: EventState::Unselected,
            });
            updates.push(match discarded {
                Some(discarded) => {
                    self.overflow = true;
                    UpdateInfo::Overflow {
                        created: id,
                        discarded,
                    }
                }
                None => UpdateInfo::Created(id),
            });
        }
        updates
    }

    /// return every event to the buffer, to be reported again
    fn reset(&mut self) {
        for event in &mut self.events {
            event.state = EventState::Unselected;
        }
    }

    /// Delete the events of a confirmed response, reporting each one and then the buffer that's
    /// left
    fn clear_written(&mut self) {
        self.callbacks.push(Event::BeginConfirm);
        let mut kept = Vec::new();
        for event in std::mem::take(&mut self.events) {
            if event.state == EventState::Written {
                self.callbacks.push(Event::Cleared(event.id));
            } else {
                kept.push(event);
            }
        }
        self.events = kept;
        if !self.is_any_full() {
            self.overflow = false;
        }
        self.callbacks.push(Event::EndConfirm(self.buffer_state()));
    }

    fn buffer_state(&self) -> BufferState {
        let classes = |class: u8| {
            self.events
                .iter()
                .filter(|event| event.class == class)
                .count()
        };
        BufferState {
            classes: ClassCount {
                num_class_1: classes(0),
                num_class_2: classes(1),
                num_class_3: classes(2),
            },
            types: TypeCount {
                num_binary_input: 0,
                num_double_bit_binary_input: 0,
                num_binary_output_status: 0,
                num_counter: self.type_count(Kind::Counter),
                num_frozen_counter: 0,
                num_analog: self.type_count(Kind::Analog),
                num_analog_output_status: 0,
                num_octet_string: 0,
            },
        }
    }

    // ---- responses ----

    fn write(&mut self, what: &str) -> Vec<u8> {
        match self.writes.pop_front() {
            Some(bytes) => bytes,
            None => panic!("{}: expected {what}, but nothing was sent", self.context),
        }
    }

    fn expect_write(&mut self, expected: &[u8], what: &str) {
        let actual = self.write(what);
        assert_eq!(
            actual, expected,
            "{}: {what}: got {actual:02X?}, expected {expected:02X?}",
            self.context
        );
    }

    /// the IIN of a response built now: the events not in this or an earlier unconfirmed response,
    /// the restart and the overflow
    fn iin(&self, iin2: u8) -> [u8; 2] {
        let mut iin1 = if self.restart { iin1::RESTART } else { 0 };
        for event in &self.events {
            if event.state != EventState::Written {
                iin1 |= 0x02 << event.class;
            }
        }
        let overflow = if self.overflow {
            iin2::EVENT_BUFFER_OVERFLOW
        } else {
            0
        };
        [iin1, iin2 | overflow]
    }

    /// Check a response that carries the selected events, the oldest first, and mark the ones it
    /// carries written. Returns the response, how many events it carries and whether those are all
    /// of them.
    fn event_response(&mut self, max_size: usize, what: &str) -> (Vec<u8>, usize, bool) {
        let bytes = self.write(what);
        assert!(
            (4..=max_size).contains(&bytes.len()),
            "{}: {what} has {} bytes: {bytes:02X?}",
            self.context,
            bytes.len()
        );
        let objects = match decode_events(&bytes[4..]) {
            Ok(objects) => objects,
            Err(err) => panic!("{}: {what}: {err}: {bytes:02X?}", self.context),
        };
        let selected: Vec<usize> = (0..self.events.len())
            .filter(|position| self.events[*position].state == EventState::Selected)
            .collect();
        assert!(
            objects.len() <= selected.len() && (selected.is_empty() || !objects.is_empty()),
            "{}: {what} carries {} events of the {} selected: {bytes:02X?}",
            self.context,
            objects.len(),
            selected.len()
        );
        for (object, position) in objects.iter().zip(&selected) {
            let event = &mut self.events[*position];
            let expected = EventObject {
                kind: event.kind,
                index: event.index,
                flags: ONLINE,
                value: event.id + 1,
            };
            assert_eq!(*object, expected, "{}: {what}", self.context);
            event.state = EventState::Written;
        }
        (bytes, objects.len(), objects.len() == selected.len())
    }

    /// Check a fragment of the response to a READ. Returns the wait for its confirm, if it asks
    /// for one.
    fn solicited_fragment(&mut self, fir: bool, seq: u8) -> Wait {
        let (bytes, count, fin) =
            self.event_response(self.setup.solicited_size, "solicited response");
        let con = count > 0 || !fin;
        let mut control = seq;
        for (bit, set) in [(0x80, fir), (0x40, fin), (0x20, con)] {
            if set {
                control |= bit;
            }
        }
        let [iin1, iin2] = self.iin(0);
        assert_eq!(
            bytes[..4],
            response_header(control, iin1, iin2),
            "{}: solicited response header: {bytes:02X?}",
            self.context
        );
        self.last_solicited = Some(bytes);
        if con {
            Wait::Solicited {
                ecsn: seq,
                fin,
                deadline: self.now + self.setup.confirm_ms,
            }
        } else {
            Wait::Idle
        }
    }

    fn respond(&mut self, seq: u8, iin2: u8, objects: &[u8], what: &str) {
        let [iin1, iin2] = self.iin(iin2);
        let expected = with_objects(solicited(seq, false, iin1, iin2), objects);
        self.expect_write(&expected, what);
        self.last_solicited = Some(expected);
    }

    // ---- the session ----

    fn read(&mut self, seq: u8, headers: &[ReadHeader]) {
        // each READ starts its selection from a reset
        self.reset();
        for header in headers {
            let mut count = 0;
            for event in &mut self.events {
                if header
                    .limit()
                    .is_some_and(|limit| count >= usize::from(limit))
                {
                    break;
                }
                if event.state == EventState::Unselected && header.matches(event) {
                    event.state = EventState::Selected;
                    count += 1;
                }
            }
        }
        self.wait = self.solicited_fragment(true, seq);
        if let Wait::Solicited { .. } = self.wait {
            self.callbacks.push(Event::EnterSolicitedConfirmWait(seq));
        }
    }

    /// Answer a request in idle or in the unsolicited confirm wait
    fn accept(&mut self, request: Request) {
        let seq = request.seq;
        let repeat = self.last_valid.as_ref() == Some(&request);
        match &request.function {
            // a repeated READ is answered as a new one
            Function::Read(headers) => self.read(seq, headers),
            _ if repeat => {
                let Some(response) = self.last_solicited.clone() else {
                    panic!("{}: a repeat with no stored response", self.context)
                };
                self.expect_write(&response, "repeated response");
            }
            Function::DelayMeasure => self.respond(
                seq,
                0,
                &[0x34, 0x02, 0x07, 0x01, 0x00, 0x00],
                "DELAY_MEASURE response",
            ),
            Function::ClearRestart => {
                self.restart = false;
                self.callbacks.push(Event::ClearRestartIIN);
                self.respond(seq, 0, &[], "WRITE response");
            }
            Function::EnableUnsolicited(named) | Function::DisableUnsolicited(named) => {
                let enable = matches!(request.function, Function::EnableUnsolicited(_));
                if !self.setup.unsolicited {
                    self.respond(
                        seq,
                        iin2::NO_FUNC_CODE_SUPPORT,
                        &[],
                        "unsupported ENABLE/DISABLE response",
                    );
                } else {
                    for (enabled, named) in self.classes.iter_mut().zip(named) {
                        if *named {
                            *enabled = enable;
                        }
                    }
                    if let Wait::Unsolicited { disabled, .. } = &mut self.wait {
                        *disabled |= !enable;
                    }
                    self.respond(seq, 0, &[], "ENABLE/DISABLE response");
                }
            }
        }
        self.last_valid = Some(request);
    }

    fn request(&mut self, request: Request) {
        match self.wait {
            Wait::Idle => {
                self.accept(request);
                self.run_idle();
            }
            Wait::Solicited { ecsn, fin, .. } => {
                if self.last_valid.as_ref() == Some(&request) {
                    // a repeated READ: the same fragment again, and the wait starts over
                    let Some(response) = self.last_solicited.clone() else {
                        panic!("{}: a solicited wait with no response", self.context)
                    };
                    self.expect_write(&response, "repeated READ response");
                    self.wait = Wait::Solicited {
                        ecsn,
                        fin,
                        deadline: self.now + self.setup.confirm_ms,
                    };
                } else {
                    // Table 6-1 rows 15 and 18: a new request ends the wait and is processed at
                    // once, before an unsolicited response
                    self.callbacks.push(Event::SolicitedConfirmWaitNewRequest);
                    self.reset();
                    self.wait = Wait::Idle;
                    self.accept(request);
                    self.run_idle();
                }
            }
            // Table 6-1 row 25: a READ waits for the end of the unsolicited confirm wait, any other
            // request discards it
            Wait::Unsolicited { .. } => match request {
                Request {
                    seq,
                    function: Function::Read(headers),
                } => self.deferred = Some((seq, headers)),
                request => {
                    self.deferred = None;
                    self.accept(request);
                }
            },
        }
    }

    fn confirm(&mut self, uns: bool, seq: u8) {
        match self.wait {
            Wait::Idle => {}
            Wait::Solicited { ecsn, fin, .. } => {
                if uns {
                    self.callbacks.push(Event::UnexpectedConfirm(true, seq));
                } else if seq != ecsn {
                    self.callbacks
                        .push(Event::WrongSolicitedConfirmSeq(ecsn, seq));
                } else {
                    self.callbacks.push(Event::SolicitedConfirmReceived(ecsn));
                    self.clear_written();
                    self.wait = if fin {
                        Wait::Idle
                    } else {
                        self.solicited_fragment(false, next_seq(ecsn))
                    };
                    self.run_idle();
                }
            }
            Wait::Unsolicited {
                seq: expected,
                null,
                ..
            } => {
                // anything but the confirm of the unsolicited response is ignored
                if uns && seq == expected {
                    self.callbacks.push(Event::UnsolicitedConfirmReceived(seq));
                    if !null {
                        self.clear_written();
                    }
                    self.unsolicited = UnsolicitedState::Ready(None);
                    self.end_unsolicited_wait();
                }
            }
        }
    }

    /// End the unsolicited confirm wait, answering a READ received during it before anything else
    fn end_unsolicited_wait(&mut self) {
        self.wait = Wait::Idle;
        if let Some((seq, headers)) = self.deferred.take() {
            self.read(seq, &headers);
            self.last_valid = Some(Request {
                seq,
                function: Function::Read(headers),
            });
        }
        self.run_idle();
    }

    /// What the outstation does in idle: send the null unsolicited response until it's confirmed,
    /// then, after any retry delay, a response with the events of the enabled classes
    fn run_idle(&mut self) {
        if !matches!(self.wait, Wait::Idle) || !self.setup.unsolicited {
            return;
        }
        match self.unsolicited {
            UnsolicitedState::NullRequired => {
                let seq = self.take_unsolicited_seq();
                let [iin1, iin2] = self.iin(0);
                let response = vec![0xF0 | seq, 0x82, iin1, iin2];
                self.expect_write(&response, "null unsolicited response");
                self.enter_unsolicited_wait(seq, true, Some(0), response);
            }
            UnsolicitedState::Ready(retry_at) => {
                if retry_at.is_some_and(|retry_at| self.now < retry_at)
                    || !self.classes.contains(&true)
                {
                    return;
                }
                // each series starts its selection from a reset
                self.reset();
                let classes = self.classes;
                let mut selected = false;
                for event in &mut self.events {
                    if classes[usize::from(event.class)] {
                        event.state = EventState::Selected;
                        selected = true;
                    }
                }
                if !selected {
                    return;
                }
                let (bytes, _, _) =
                    self.event_response(self.setup.unsolicited_size, "unsolicited response");
                let seq = self.take_unsolicited_seq();
                let [iin1, iin2] = self.iin(0);
                assert_eq!(
                    bytes[..4],
                    [0xF0 | seq, 0x82, iin1, iin2],
                    "{}: unsolicited response header: {bytes:02X?}",
                    self.context
                );
                self.enter_unsolicited_wait(seq, false, self.setup.max_retries, bytes);
            }
        }
    }

    fn take_unsolicited_seq(&mut self) -> u8 {
        let seq = self.next_unsolicited_seq;
        self.next_unsolicited_seq = next_seq(seq);
        seq
    }

    fn enter_unsolicited_wait(
        &mut self,
        seq: u8,
        null: bool,
        retries: Option<usize>,
        response: Vec<u8>,
    ) {
        self.callbacks.push(Event::EnterUnsolicitedConfirmWait(seq));
        self.last_unsolicited = Some(response);
        self.wait = Wait::Unsolicited {
            seq,
            null,
            retries,
            deadline: self.now + self.setup.confirm_ms,
            disabled: false,
        };
    }

    /// Move the clock, firing each deadline on the way in order
    fn advance(&mut self, ms: u64) {
        let target = self.now + ms;
        while let Some(deadline) = self.next_deadline().filter(|deadline| *deadline <= target) {
            self.now = deadline;
            self.timeout();
        }
        self.now = target;
    }

    fn timeout(&mut self) {
        match self.wait {
            Wait::Idle => self.run_idle(),
            Wait::Solicited { ecsn, .. } => {
                self.callbacks.push(Event::SolicitedConfirmTimeout(ecsn));
                self.reset();
                self.wait = Wait::Idle;
                self.run_idle();
            }
            Wait::Unsolicited {
                seq,
                null,
                retries,
                disabled,
                ..
            } => {
                let (retry, retries) = match retries {
                    None => (true, None),
                    Some(0) => (false, Some(0)),
                    Some(left) => (true, Some(left - 1)),
                };
                // TB2015-002a rules 15 and 16: no retry after a DISABLE or with a READ waiting
                let retry = retry && !disabled && self.deferred.is_none();
                self.callbacks
                    .push(Event::UnsolicitedConfirmTimeout(seq, retry));
                if retry {
                    let Some(response) = self.last_unsolicited.clone() else {
                        panic!("{}: an unsolicited wait with no response", self.context)
                    };
                    self.expect_write(&response, "unsolicited retry");
                    self.wait = Wait::Unsolicited {
                        seq,
                        null,
                        retries,
                        deadline: self.now + self.setup.confirm_ms,
                        disabled,
                    };
                } else {
                    self.reset();
                    self.unsolicited = if null {
                        UnsolicitedState::NullRequired
                    } else {
                        UnsolicitedState::Ready(Some(self.now + self.setup.retry_ms))
                    };
                    self.end_unsolicited_wait();
                }
            }
        }
    }

    /// A new connection forgets the last request, a deferred READ and the stored responses, and
    /// returns every event to the buffer. The unsolicited state, the enabled classes and the restart
    /// IIN stay.
    fn reconnect(&mut self) {
        self.reset();
        self.last_valid = None;
        self.deferred = None;
        self.last_solicited = None;
        self.last_unsolicited = None;
        self.wait = Wait::Idle;
        self.run_idle();
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ConfirmChoice {
    /// the sequence number and UNS bit of the response awaited
    Expected,
    /// another sequence number
    Offset(u8),
    /// the other UNS bit
    OtherKind,
}

impl ConfirmChoice {
    fn seq(self, base: u8) -> u8 {
        match self {
            ConfirmChoice::Expected | ConfirmChoice::OtherKind => base,
            ConfirmChoice::Offset(offset) => (base + offset) % 16,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum AdvanceChoice {
    /// to 1 ms before the next deadline
    BeforeDeadline,
    AtDeadline,
    AfterDeadline,
    Millis(u64),
}

#[derive(Clone, Debug)]
enum Op {
    Update {
        kind: Kind,
        index: u16,
        count: u64,
    },
    /// a request with the next sequence number
    Request(Function),
    /// a request with the last request's sequence number
    SameSeq(Function),
    /// send the last request's bytes again
    Repeat,
    SolicitedConfirm(ConfirmChoice),
    UnsolicitedConfirm(ConfirmChoice),
    Advance(AdvanceChoice),
    Reconnect,
}

fn kind() -> impl Strategy<Value = Kind> {
    prop_oneof![Just(Kind::Counter), Just(Kind::Analog)]
}

fn classes() -> impl Strategy<Value = [bool; 3]> {
    (1u8..8).prop_map(|mask| [mask & 1 != 0, mask & 2 != 0, mask & 4 != 0])
}

fn read_header() -> impl Strategy<Value = ReadHeader> {
    let limit = || prop_oneof![2 => Just(None), 1 => (1u8..6).prop_map(Some)];
    prop_oneof![
        3 => (0u8..3, limit()).prop_map(|(class, limit)| ReadHeader::Class(class, limit)),
        1 => (kind(), limit()).prop_map(|(kind, limit)| ReadHeader::Type(kind, limit)),
    ]
}

fn confirm_choice() -> impl Strategy<Value = ConfirmChoice> {
    prop_oneof![
        6 => Just(ConfirmChoice::Expected),
        1 => (1u8..16).prop_map(ConfirmChoice::Offset),
        1 => Just(ConfirmChoice::OtherKind),
    ]
}

fn advance_choice() -> impl Strategy<Value = AdvanceChoice> {
    prop_oneof![
        2 => Just(AdvanceChoice::BeforeDeadline),
        2 => Just(AdvanceChoice::AtDeadline),
        2 => Just(AdvanceChoice::AfterDeadline),
        1 => Just(AdvanceChoice::Millis(1)),
        1 => (0u64..5000).prop_map(AdvanceChoice::Millis),
    ]
}

fn update_op() -> impl Strategy<Value = Op> {
    // a few events, or enough to fill more than one response of the minimum size
    let count = prop_oneof![3 => 1u64..5, 1 => LARGE_UPDATE];
    (kind(), 0..POINTS, count).prop_map(|(kind, index, count)| Op::Update { kind, index, count })
}

/// enough events to fill more than one response of the minimum size
const LARGE_UPDATE: Range<u64> = 30..46;

fn large_update_op() -> impl Strategy<Value = Op> {
    (kind(), 0..POINTS, LARGE_UPDATE).prop_map(|(kind, index, count)| Op::Update {
        kind,
        index,
        count,
    })
}

fn read_every_class() -> Function {
    Function::Read((0..3).map(|class| ReadHeader::Class(class, None)).collect())
}

fn read_function() -> impl Strategy<Value = Function> {
    prop::collection::vec(read_header(), 1..4).prop_map(Function::Read)
}

/// a request's function, a READ `reads` times and an ENABLE_UNSOLICITED `enables` times as likely as
/// each of the others
fn function(reads: u32, enables: u32) -> impl Strategy<Value = Function> {
    prop_oneof![
        reads => read_function(),
        enables => classes().prop_map(Function::EnableUnsolicited),
        1 => classes().prop_map(Function::DisableUnsolicited),
        1 => Just(Function::DelayMeasure),
        1 => Just(Function::ClearRestart),
    ]
}

fn read_op() -> impl Strategy<Value = Op> {
    read_function().prop_map(Op::Request)
}

/// a new request, one with the last request's sequence number, or a repeat, each function as
/// likely as the others
fn request_op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => function(1, 1).prop_map(Op::Request),
        2 => function(1, 1).prop_map(Op::SameSeq),
        1 => Just(Op::Repeat),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        6 => update_op(),
        10 => function(5, 2).prop_map(Op::Request),
        2 => function(5, 2).prop_map(Op::SameSeq),
        4 => confirm_choice().prop_map(Op::SolicitedConfirm),
        4 => confirm_choice().prop_map(Op::UnsolicitedConfirm),
        2 => Just(Op::Repeat),
        4 => advance_choice().prop_map(Op::Advance),
        2 => Just(Op::Reconnect),
    ]
}

/// A READ, an update and any request. When the READ enters the solicited confirm wait and the
/// update is in a class enabled for unsolicited responses, the request is new while an unsolicited
/// response is due (Table 6-1 rows 15 and 18), or a repeat.
fn request_after_read_and_update() -> impl Strategy<Value = Vec<Op>> {
    (read_op(), update_op(), request_op()).prop_map(|(read, update, next)| vec![read, update, next])
}

/// The confirm of the null unsolicited response, unsolicited responses enabled for every class but
/// one, events in that class and a READ of them, which enters the solicited confirm wait, then an
/// event in an enabled class, due as an unsolicited response, and any request
fn due_in_the_solicited_wait() -> impl Strategy<Value = Vec<Op>> {
    (kind(), 0..POINTS, 1u64..5, kind(), 0..POINTS, request_op()).prop_map(
        |(kind, index, count, due_kind, due_index, request)| {
            let class = kind.class(index);
            // the first point from `due_index` on in another class
            let due_index = (0..POINTS)
                .map(|offset| (due_index + offset) % POINTS)
                .find(|index| due_kind.class(*index) != class)
                .unwrap_or(due_index);
            vec![
                Op::UnsolicitedConfirm(ConfirmChoice::Expected),
                Op::Request(Function::EnableUnsolicited([0, 1, 2].map(|c| c != class))),
                Op::Update { kind, index, count },
                Op::Request(Function::Read(vec![ReadHeader::Class(class, None)])),
                Op::Update {
                    kind: due_kind,
                    index: due_index,
                    count: 1,
                },
                request,
            ]
        },
    )
}

/// Large updates, a READ, the confirm of its first fragment and a repeat of the READ: a repeat of a
/// later fragment of the series, and of a middle one when there are three or more
fn repeat_of_a_later_fragment() -> impl Strategy<Value = Vec<Op>> {
    let read = prop_oneof![read_op(), Just(Op::Request(read_every_class()))];
    (prop::collection::vec(large_update_op(), 2..5), read).prop_map(|(mut ops, read)| {
        ops.extend([
            read,
            Op::SolicitedConfirm(ConfirmChoice::Expected),
            Op::Repeat,
        ]);
        ops
    })
}

/// A READ, a wait shorter than the confirm timeout, a repeat of the READ and a wait to the next
/// deadline: the timeout of the solicited confirm wait the repeat restarted
fn timeout_of_a_restarted_wait() -> impl Strategy<Value = Vec<Op>> {
    let advance = prop_oneof![
        Just(AdvanceChoice::BeforeDeadline),
        Just(AdvanceChoice::Millis(1))
    ];
    (read_op(), advance).prop_map(|(read, advance)| {
        vec![
            read,
            Op::Advance(advance),
            Op::Repeat,
            Op::Advance(AdvanceChoice::AtDeadline),
        ]
    })
}

/// An update, a DISABLE_UNSOLICITED and waits to the next two deadlines: the timeout of an
/// unsolicited confirm wait after a DISABLE, and the next series after the retry delay
fn timeout_after_disable() -> impl Strategy<Value = Vec<Op>> {
    (update_op(), classes()).prop_map(|(update, classes)| {
        vec![
            update,
            Op::Request(Function::DisableUnsolicited(classes)),
            Op::Advance(AdvanceChoice::AtDeadline),
            Op::Advance(AdvanceChoice::AtDeadline),
        ]
    })
}

/// Single ops, and runs of ops that reach states single ops rarely do
fn ops() -> impl Strategy<Value = Vec<Op>> {
    let run = prop_oneof![
        16 => op().prop_map(|op| vec![op]),
        2 => request_after_read_and_update(),
        3 => due_in_the_solicited_wait(),
        1 => repeat_of_a_later_fragment(),
        1 => timeout_of_a_restarted_wait(),
        1 => timeout_after_disable(),
    ];
    prop::collection::vec(run, 1..40).prop_map(|runs| runs.concat())
}

fn setup() -> impl Strategy<Value = Setup> {
    // small limits evict often, middle ones evict during a series of several fragments
    let limit = || prop_oneof![1 => 1u16..5, 1 => 35u16..60, 1 => Just(100u16)];
    let size = || prop_oneof![Just(BufferSize::<249, 2048>::MIN), Just(2048)];
    (
        prop::bool::weighted(0.75),
        prop_oneof![Just(None), (0usize..3).prop_map(Some)],
        prop_oneof![Just(1u64), 2u64..3000],
        0u64..3000,
        limit(),
        limit(),
        size(),
        size(),
    )
        .prop_map(
            |(
                unsolicited,
                max_retries,
                confirm_ms,
                retry_ms,
                max_counter,
                max_analog,
                solicited_size,
                unsolicited_size,
            )| Setup {
                unsolicited,
                max_retries,
                confirm_ms,
                // distinct from the confirm timeout, so that the two can't be confused
                retry_ms: if retry_ms == confirm_ms {
                    retry_ms + 1
                } else {
                    retry_ms
                },
                max_counter,
                max_analog,
                solicited_size,
                unsolicited_size,
            },
        )
}

fn config(setup: Setup) -> OutstationConfig {
    let mut config = without_keep_alive(get_default_unsolicited_config());
    config.features.unsolicited = if setup.unsolicited {
        Feature::Enabled
    } else {
        Feature::Disabled
    };
    config.confirm_timeout = Timeout::from_millis(setup.confirm_ms).unwrap();
    config.unsolicited_retry_delay = Duration::from_millis(setup.retry_ms);
    config.max_unsolicited_retries = setup.max_retries;
    config.event_buffer_config = EventBufferConfig {
        max_counter: setup.max_counter,
        max_analog: setup.max_analog,
        ..EventBufferConfig::all_types(0)
    };
    config.solicited_buffer_size = BufferSize::new(setup.solicited_size).unwrap();
    config.unsolicited_buffer_size = BufferSize::new(setup.unsolicited_size).unwrap();
    config
}

fn add_points(db: &mut Database) {
    for index in 0..POINTS {
        db.add(
            index,
            Some(event_class(Kind::Counter.class(index))),
            CounterConfig::default(),
        );
        db.add(
            index,
            Some(event_class(Kind::Analog.class(index))),
            AnalogInputConfig::default(),
        );
    }
}

/// update the point with the event `id`, whose value is `id + 1`
fn update(db: &mut Database, kind: Kind, index: u16, id: u64) -> UpdateInfo {
    let value = u32::try_from(id + 1).unwrap();
    let time = Time::synchronized(0);
    let options = UpdateOptions::detect_event();
    match kind {
        Kind::Counter => db.update2(index, &Counter::new(value, Flags::ONLINE, time), options),
        Kind::Analog => db.update2(
            index,
            &AnalogInput::new(f64::from(value), Flags::ONLINE, time),
            options,
        ),
    }
}

/// Give the input to the outstation, and check what it did against the model
async fn step(
    mut harness: OutstationHarness,
    model: &mut Model,
    start: Instant,
    input: Input,
) -> OutstationHarness {
    let mut updates = Vec::new();
    match &input {
        Input::Start => {}
        Input::Update { kind, index, ids } => {
            updates = harness.handle.database.transaction(|db| {
                ids.clone()
                    .map(|id| update(db, *kind, *index, id))
                    .collect::<Vec<_>>()
            });
        }
        Input::Request(request) => harness.send_and_process(&request.bytes()).await,
        Input::Confirm { uns: false, seq } => harness.send_and_process(&confirm(*seq)).await,
        Input::Confirm { uns: true, seq } => {
            harness.send_and_process(&unsolicited_confirm(*seq)).await
        }
        // the paused clock moves to each timer due on the way, so the outstation handles each one
        // at its deadline
        Input::Advance(ms) => tokio::time::sleep(Duration::from_millis(*ms)).await,
        Input::Reconnect => harness = harness.drop_link().await.reconnect(),
    }
    match input {
        // the outstation has answered once `send_and_process` returns (see `pop_response`)
        Input::Request(_) | Input::Confirm { .. } => {}
        // the outstation was woken, by its start, the database or a timer due when the sleep ended,
        // but hasn't run. A yield lets every task already woken run before this one (see
        // `tokio::task::yield_now`), and the outstation handles the wake-up in that one poll, as
        // `pop_response` describes
        Input::Start | Input::Update { .. } | Input::Advance(_) | Input::Reconnect => {
            tokio::task::yield_now().await
        }
    }

    // every response and callback since the last input: the model accepts no more than it expects
    let writes = harness.drain_responses();
    let callbacks = harness.drain_events();

    let outcome = model.apply(&input, writes);
    assert_eq!(callbacks, outcome.callbacks, "{}: callbacks", model.context);
    assert_eq!(updates, outcome.updates, "{}: updates", model.context);
    assert_eq!(
        start.elapsed(),
        Duration::from_millis(model.now),
        "{}: clock",
        model.context
    );
    harness
}

async fn check_model(setup: Setup, ops: Vec<Op>) {
    let mut harness = new_harness(config(setup));
    harness.handle.database.transaction(add_points);
    let start = Instant::now();
    let mut model = Model::new(setup);

    harness = step(harness, &mut model, start, Input::Start).await;
    for op in &ops {
        if let Some(input) = model.resolve(op) {
            harness = step(harness, &mut model, start, input).await;
        }
    }

    // Every event left is reported and confirmed exactly once. The drain adds no events, and the
    // model requires each response to carry some of the events selected for it, so every confirm
    // deletes at least one event but two: the first may confirm a response whose events were all
    // evicted before the drain, and one may confirm the null response, which a deferred READ's
    // answer can hold back until then. The DISABLE and the READ make two more inputs.
    let limit = model.events.len() + 4;
    for _ in 0..limit {
        let Some(op) = model.drain_op() else {
            break;
        };
        let input = model.resolve(&op).expect("the drain's inputs always apply");
        harness = step(harness, &mut model, start, input).await;
    }
    assert!(
        model.drain_op().is_none(),
        "the buffer still holds {} events after {limit} inputs of the drain",
        model.events.len()
    );

    // and the outstation has nothing more to say
    tokio::task::yield_now().await;
    harness.expect_no_response();
    harness.check_no_events();
}

proptest! {
    #![proptest_config(crate::util::test_support::proptest_config(128))]

    #[test]
    fn session_follows_the_event_and_confirm_model(
        setup in setup(),
        ops in ops(),
    ) {
        run_paused(check_model(setup, ops));
    }
}

/// A case with unsolicited responses enabled and no retries, whose ops end in the solicited confirm
/// wait of a READ while an unsolicited response is due, then send `request`
fn due_in_the_solicited_wait_case(request: Op) -> (Setup, Vec<Op>) {
    let setup = Setup {
        unsolicited: true,
        max_retries: Some(0),
        confirm_ms: 2047,
        retry_ms: 2,
        max_counter: 1,
        max_analog: 1,
        solicited_size: BufferSize::<249, 2048>::MIN,
        unsolicited_size: BufferSize::<249, 2048>::MIN,
    };
    let ops = vec![
        // the null unsolicited response
        Op::UnsolicitedConfirm(ConfirmChoice::Expected),
        Op::Request(Function::EnableUnsolicited([false, false, true])),
        // a class 3 event, sent unsolicited
        Op::Update {
            kind: Kind::Counter,
            index: 2,
            count: 1,
        },
        // no retries: the retry delay starts
        Op::Advance(AdvanceChoice::AtDeadline),
        // answered with CON, entering the solicited confirm wait
        Op::Request(Function::Read(vec![ReadHeader::Class(2, None)])),
        // past the retry delay, so an unsolicited response is due
        Op::Advance(AdvanceChoice::BeforeDeadline),
        request,
    ];
    (setup, ops)
}

/// The property's first failure: a new READ in the solicited confirm wait, after the unsolicited
/// retry delay has passed, is answered before the unsolicited response that's due (Table 6-1 row
/// 15)
#[test]
fn new_read_in_the_solicited_wait_comes_before_a_due_unsolicited_response() {
    let (setup, ops) =
        due_in_the_solicited_wait_case(Op::Request(Function::Read(vec![ReadHeader::Class(
            0, None,
        )])));
    run_paused(check_model(setup, ops));
}

/// Like a READ, a new request of another function in the solicited confirm wait is answered
/// before the unsolicited response that's due (Table 6-1 row 15)
#[test]
fn new_non_read_in_the_solicited_wait_comes_before_a_due_unsolicited_response() {
    let (setup, ops) = due_in_the_solicited_wait_case(Op::Request(Function::DelayMeasure));
    run_paused(check_model(setup, ops));
}

/// a case without unsolicited responses, with room for 100 counter events
fn solicited_only() -> Setup {
    Setup {
        unsolicited: false,
        max_retries: None,
        confirm_ms: 1000,
        retry_ms: 5000,
        max_counter: 100,
        max_analog: 1,
        solicited_size: BufferSize::<249, 2048>::MIN,
        unsolicited_size: BufferSize::<249, 2048>::MIN,
    }
}

/// class 1 events, from counter 0
fn class_1_events(count: u64) -> Op {
    Op::Update {
        kind: Kind::Counter,
        index: 0,
        count,
    }
}

/// a READ of the class 1 events
fn class_1_read() -> Op {
    Op::Request(Function::Read(vec![ReadHeader::Class(0, None)]))
}

/// A request in the solicited confirm wait with the READ's sequence number but other headers is a
/// new request, not a repeat (Table 6-1 row 18): it ends the wait and is answered
#[test]
fn request_with_the_reads_sequence_number_and_other_headers_is_new() {
    let ops = vec![
        class_1_events(3),
        Op::Request(Function::Read(vec![ReadHeader::Class(0, None)])),
        // one event, in a new wait for the same sequence number
        Op::SameSeq(Function::Read(vec![ReadHeader::Class(0, Some(1))])),
        Op::SolicitedConfirm(ConfirmChoice::Expected),
        // and a request of another function
        class_1_events(1),
        Op::Request(Function::Read(vec![ReadHeader::Class(0, None)])),
        Op::SameSeq(Function::DelayMeasure),
    ];
    run_paused(check_model(solicited_only(), ops));
}

/// A repeated READ while the first, a middle or the last fragment of its series waits for its
/// confirm gets that fragment's bytes again, as often as it's repeated, and the series goes on from
/// there
#[test]
fn repeated_read_gets_the_fragment_waiting() {
    let ops = vec![
        // three fragments, the first two of the minimum size and the last shorter
        class_1_events(80),
        class_1_read(),
        Op::Repeat,
        Op::SolicitedConfirm(ConfirmChoice::Expected),
        Op::Repeat,
        Op::Repeat,
        Op::SolicitedConfirm(ConfirmChoice::Expected),
        Op::Repeat,
        Op::SolicitedConfirm(ConfirmChoice::Expected),
    ];
    run_paused(check_model(solicited_only(), ops));
}

/// A repeated READ in the solicited confirm wait restarts the confirm timeout: the wait doesn't end
/// at the first deadline, but at the one the repeat set
#[test]
fn repeated_read_restarts_the_solicited_confirm_timeout() {
    let ops = vec![
        class_1_events(1),
        Op::Request(Function::Read(vec![ReadHeader::Class(0, None)])),
        Op::Advance(AdvanceChoice::Millis(400)),
        Op::Repeat,
        // past the first deadline
        Op::Advance(AdvanceChoice::Millis(800)),
        Op::Advance(AdvanceChoice::BeforeDeadline),
        Op::Advance(AdvanceChoice::AtDeadline),
    ];
    run_paused(check_model(solicited_only(), ops));
}

/// A case with unsolicited responses and room for 100 counter events. The confirm timeout, the
/// retry delay and the 5 s select timeout all differ, so that none can stand in for another
fn unsolicited_with_retries(max_retries: Option<usize>) -> Setup {
    Setup {
        unsolicited: true,
        max_retries,
        confirm_ms: 2000,
        retry_ms: 11000,
        max_counter: 100,
        max_analog: 1,
        solicited_size: BufferSize::<249, 2048>::MIN,
        unsolicited_size: BufferSize::<249, 2048>::MIN,
    }
}

/// longer than the confirm timeout and the retry delay together
const SILENCE: AdvanceChoice = AdvanceChoice::Millis(30_000);

/// to 1 ms before the next deadline, and then to it
fn to_the_deadline() -> Vec<Op> {
    vec![
        Op::Advance(AdvanceChoice::BeforeDeadline),
        Op::Advance(AdvanceChoice::AtDeadline),
    ]
}

/// The confirm of the null unsolicited response, unsolicited responses enabled for every class, and
/// a class 1 event, sent in an unsolicited response that waits for its confirm
fn unsolicited_class_1_event() -> Vec<Op> {
    vec![
        Op::UnsolicitedConfirm(ConfirmChoice::Expected),
        Op::Request(Function::EnableUnsolicited([true; 3])),
        class_1_events(1),
    ]
}

fn disable_unsolicited(classes: [bool; 3]) -> Op {
    Op::Request(Function::DisableUnsolicited(classes))
}

/// The null unsolicited response isn't retried but sent again with the next sequence number each
/// time its confirm wait times out, whatever the retry limit
#[test]
fn null_unsolicited_response_is_sent_again_with_the_next_sequence_number() {
    for max_retries in [None, Some(2)] {
        let ops = [to_the_deadline(), to_the_deadline()].concat();
        run_paused(check_model(unsolicited_with_retries(max_retries), ops));
    }
}

/// With no retry limit, an unsolicited response is sent again byte for byte each time its confirm
/// wait times out, until the master confirms it
#[test]
fn unsolicited_response_is_sent_again_until_it_is_confirmed() {
    let ops = [
        unsolicited_class_1_event(),
        to_the_deadline(),
        to_the_deadline(),
        to_the_deadline(),
        vec![Op::UnsolicitedConfirm(ConfirmChoice::Expected)],
    ]
    .concat();
    run_paused(check_model(unsolicited_with_retries(None), ops));
}

/// After its last retry, an unsolicited series ends at the confirm timeout, and the next series
/// starts with the next sequence number once the retry delay has passed
#[test]
fn next_unsolicited_series_starts_after_the_last_retry_and_the_retry_delay() {
    for retries in 0..3 {
        let mut ops = unsolicited_class_1_event();
        // each retry, the end of the series and the end of the retry delay
        for _ in 0..retries + 2 {
            ops.extend(to_the_deadline());
        }
        ops.push(Op::UnsolicitedConfirm(ConfirmChoice::Expected));
        run_paused(check_model(unsolicited_with_retries(Some(retries)), ops));
    }
}

/// In the retry delay that follows an unconfirmed unsolicited response, that response's event is
/// reported in IIN again, and a READ reports it with the one since, and its confirm deletes both
#[test]
fn read_in_the_retry_delay_reports_the_events_of_the_unconfirmed_unsolicited_response() {
    let ops = [
        unsolicited_class_1_event(),
        // no retries: the retry delay starts
        to_the_deadline(),
        vec![
            Op::Request(Function::DelayMeasure),
            class_1_events(1),
            class_1_read(),
            Op::SolicitedConfirm(ConfirmChoice::Expected),
            class_1_read(),
        ],
    ]
    .concat();
    run_paused(check_model(unsolicited_with_retries(Some(0)), ops));
}

/// A DISABLE_UNSOLICITED in the confirm wait of the null unsolicited response, or of one with an
/// event, doesn't end the wait: the confirm that follows does, deleting the event, and nothing is
/// sent after it
#[test]
fn confirm_after_disable_unsolicited_ends_the_unsolicited_confirm_wait() {
    for wait in [Vec::new(), unsolicited_class_1_event()] {
        let ops = [
            wait,
            vec![
                disable_unsolicited([true; 3]),
                Op::UnsolicitedConfirm(ConfirmChoice::Expected),
                Op::Advance(SILENCE),
                class_1_read(),
            ],
        ]
        .concat();
        run_paused(check_model(unsolicited_with_retries(None), ops));
    }
}

/// After a DISABLE_UNSOLICITED, the confirm wait of the null unsolicited response times out and the
/// null response is sent again with the next sequence number. One with an event times out without
/// a retry, despite no retry limit, and no other series starts: the event is left for a READ
#[test]
fn timeout_after_disable_unsolicited_ends_the_unsolicited_confirm_wait_without_a_retry() {
    let null = [vec![disable_unsolicited([true; 3])], to_the_deadline()].concat();
    let data = [
        unsolicited_class_1_event(),
        vec![disable_unsolicited([true; 3])],
        to_the_deadline(),
        vec![Op::Advance(SILENCE), class_1_read()],
    ]
    .concat();
    for ops in [null, data] {
        run_paused(check_model(unsolicited_with_retries(None), ops));
    }
}

/// A DISABLE_UNSOLICITED of another class than the unsolicited response's event ends its retries:
/// the wait times out without the retry left, and as the event's class is still enabled, the next
/// series starts after the retry delay
#[test]
fn disable_unsolicited_of_another_class_ends_the_unsolicited_retries() {
    let ops = [
        unsolicited_class_1_event(),
        // the first retry
        vec![
            Op::Advance(AdvanceChoice::AtDeadline),
            disable_unsolicited([false, true, false]),
        ],
        // the end of the series, and the next one
        to_the_deadline(),
        to_the_deadline(),
        vec![Op::UnsolicitedConfirm(ConfirmChoice::Expected)],
    ]
    .concat();
    run_paused(check_model(unsolicited_with_retries(Some(2)), ops));
}

/// The solicited confirm wait ends at the confirm timeout, not before
#[test]
fn solicited_confirm_wait_ends_at_the_confirm_timeout() {
    let ops = [vec![class_1_events(1), class_1_read()], to_the_deadline()].concat();
    run_paused(check_model(solicited_only(), ops));
}

/// In the solicited confirm wait, a confirm with another sequence number and one with the UNS bit
/// are reported and ignored, and the expected confirm still deletes the response's event
#[test]
fn wrong_confirms_in_the_solicited_confirm_wait_are_ignored() {
    let ops = vec![
        class_1_events(1),
        class_1_read(),
        Op::SolicitedConfirm(ConfirmChoice::Offset(1)),
        Op::SolicitedConfirm(ConfirmChoice::OtherKind),
        Op::SolicitedConfirm(ConfirmChoice::Expected),
        // whose IIN no longer reports it
        Op::Request(Function::DelayMeasure),
    ];
    run_paused(check_model(solicited_only(), ops));
}

/// In the confirm wait of the null unsolicited response, and of one with an event, a confirm with
/// another sequence number and one without the UNS bit are ignored without a callback, and the
/// expected confirm still ends the wait
#[test]
fn wrong_confirms_in_the_unsolicited_confirm_wait_are_ignored() {
    let wrong = || {
        vec![
            Op::UnsolicitedConfirm(ConfirmChoice::Offset(1)),
            Op::UnsolicitedConfirm(ConfirmChoice::OtherKind),
        ]
    };
    let ops = [
        wrong(),
        unsolicited_class_1_event(),
        wrong(),
        vec![Op::UnsolicitedConfirm(ConfirmChoice::Expected)],
    ]
    .concat();
    run_paused(check_model(unsolicited_with_retries(None), ops));
}

/// The unsolicited response that becomes due in the solicited confirm wait is sent once the wait
/// ends, by its confirm or at its timeout
#[test]
fn end_of_the_solicited_confirm_wait_releases_a_due_unsolicited_response() {
    // unsolicited responses for class 2 only, a READ of a class 1 event that waits for its confirm,
    // then a class 2 event
    let due = vec![
        Op::UnsolicitedConfirm(ConfirmChoice::Expected),
        Op::Request(Function::EnableUnsolicited([false, true, false])),
        class_1_events(1),
        class_1_read(),
        Op::Update {
            kind: Kind::Counter,
            index: 1,
            count: 1,
        },
    ];
    for end in [
        vec![Op::SolicitedConfirm(ConfirmChoice::Expected)],
        to_the_deadline(),
    ] {
        let ops = [due.clone(), end].concat();
        run_paused(check_model(unsolicited_with_retries(None), ops));
    }
}
