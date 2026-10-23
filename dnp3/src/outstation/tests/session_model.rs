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
//!   wait. Elsewhere a repeated READ is answered as a new one, a deliberate deviation.
//!
//! The outstation has handled each input when the test looks at what it did, on the guarantee
//! that `pop_response` describes: the outstation has answered a request once the test sees it read,
//! and an input that only wakes the outstation (its start, an update, a timer, a new connection)
//! is handled in the outstation's next poll, which a yield lets run first. Time only moves when the
//! test sleeps, which lets the paused clock fire the outstation's timers on the way, in order.
//! After each input the test takes every response and callback, so the model accepts no more than
//! it expects, and checks that the clock agrees with the model's.
//!
//! [`Model::covers`] leaves out two inputs:
//! - a READ during the unsolicited confirm wait. The outstation defers it until the wait ends
//!   (Table 6-1 row 25), which this model doesn't follow: it's a gap in coverage, not a known bug,
//!   for the full session model (stage 2) to close;
//! - a new request during the solicited confirm wait while an unsolicited response is due. Table 6-1
//!   rows 15 and 18 process the request at once, as the model does, but the outstation sends the
//!   unsolicited response first and then takes the request in that response's confirm wait,
//!   deferring a READ. This is a confirmed deviation, being fixed on the branch
//!   fix/sol-wait-new-request, and the exclusion goes once that fix lands.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the model adds small sequence numbers, ids, counts and millisecond times"
)]

use std::collections::VecDeque;
use std::future::Future;
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
use crate::util::test_support::watchdog;

/// points of each kind, at indices 0 to 3
const POINTS: u16 = 4;

const IIN1_RESTART: u8 = 0x80;
const IIN2_NO_FUNC_CODE_SUPPORT: u8 = 0x01;
const IIN2_EVENT_BUFFER_OVERFLOW: u8 = 0x08;

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
        let classes = |code: u8, classes: &[bool; 3]| {
            let mut bytes = vec![control, code];
            for (class, _) in (0u8..).zip(classes).filter(|(_, named)| **named) {
                bytes.extend([60, class + 2, 0x06]);
            }
            bytes
        };
        match &self.function {
            Function::Read(headers) => {
                let mut bytes = vec![control, 0x01];
                for header in headers {
                    header.write(&mut bytes);
                }
                bytes
            }
            Function::DelayMeasure => vec![control, 0x17],
            Function::ClearRestart => vec![control, 0x02, 80, 1, 0x00, 7, 7, 0x00],
            Function::EnableUnsolicited(named) => classes(0x14, named),
            Function::DisableUnsolicited(named) => classes(0x15, named),
        }
    }

    fn is_read(&self) -> bool {
        matches!(self.function, Function::Read(_))
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

impl Input {
    fn confirm_bytes(uns: bool, seq: u8) -> [u8; 2] {
        let control = if uns { 0xD0 } else { 0xC0 };
        [control | seq, 0x00]
    }
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
            Op::Read(headers) => self.new_request(Function::Read(headers.clone())),
            Op::DelayMeasure => self.new_request(Function::DelayMeasure),
            Op::ClearRestart => self.new_request(Function::ClearRestart),
            Op::Enable(classes) => self.new_request(Function::EnableUnsolicited(*classes)),
            Op::Disable(classes) => self.new_request(Function::DisableUnsolicited(*classes)),
            Op::Repeat => {
                let request = self.last_request.clone()?;
                self.covers(&request).then_some(Input::Request(request))
            }
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

    fn new_request(&mut self, function: Function) -> Option<Input> {
        let request = Request {
            seq: self.next_request_seq,
            function,
        };
        if !self.covers(&request) {
            return None;
        }
        self.next_request_seq = next_seq(self.next_request_seq);
        Some(Input::Request(request))
    }

    /// Whether the model covers the request in the current state (see the module documentation)
    fn covers(&self, request: &Request) -> bool {
        match self.wait {
            Wait::Idle => true,
            // a deferred READ: stage 2
            Wait::Unsolicited { .. } => !request.is_read(),
            // a new request while an unsolicited response is due: the deviation from Table 6-1
            // row 15 being fixed on fix/sol-wait-new-request
            Wait::Solicited { .. } => {
                self.last_valid.as_ref() == Some(request) || !self.sends_unsolicited_from_reset()
            }
        }
    }

    /// whether the outstation would send an unsolicited response from idle once every event is
    /// back in the buffer
    fn sends_unsolicited_from_reset(&self) -> bool {
        if !self.setup.unsolicited {
            return false;
        }
        match self.unsolicited {
            UnsolicitedState::NullRequired => true,
            UnsolicitedState::Ready(retry_at) => {
                !retry_at.is_some_and(|retry_at| self.now < retry_at)
                    && self.events.iter().any(|event| self.enabled(event))
            }
        }
    }

    fn enabled(&self, event: &Buffered) -> bool {
        self.classes[usize::from(event.class)]
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
                Op::Disable([true; 3])
            }
            Wait::Idle if self.events.is_empty() => return None,
            Wait::Idle => Op::Read((0..3).map(|class| ReadHeader::Class(class, None)).collect()),
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
        let mut iin1 = if self.restart { IIN1_RESTART } else { 0 };
        for event in &self.events {
            if event.state != EventState::Written {
                iin1 |= 0x02 << event.class;
            }
        }
        let overflow = if self.overflow {
            IIN2_EVENT_BUFFER_OVERFLOW
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
            [control, 0x81, iin1, iin2],
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
        let mut expected = vec![0xC0 | seq, 0x81, iin1, iin2];
        expected.extend_from_slice(objects);
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
                        IIN2_NO_FUNC_CODE_SUPPORT,
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
            Wait::Unsolicited { .. } => self.accept(request),
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
                    self.wait = Wait::Idle;
                    self.run_idle();
                }
            }
        }
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
                let retry = retry && !disabled;
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
                    self.wait = Wait::Idle;
                    self.unsolicited = if null {
                        UnsolicitedState::NullRequired
                    } else {
                        UnsolicitedState::Ready(Some(self.now + self.setup.retry_ms))
                    };
                    self.run_idle();
                }
            }
        }
    }

    /// A new connection forgets the last request and the stored responses, and returns every event
    /// to the buffer. The unsolicited state, the enabled classes and the restart IIN stay.
    fn reconnect(&mut self) {
        self.reset();
        self.last_valid = None;
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
    Read(Vec<ReadHeader>),
    DelayMeasure,
    ClearRestart,
    Enable([bool; 3]),
    Disable([bool; 3]),
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

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        // a few events, or enough to fill more than one response of the minimum size
        6 => (kind(), 0..POINTS, prop_oneof![3 => 1u64..5, 1 => 30u64..46])
            .prop_map(|(kind, index, count)| Op::Update { kind, index, count }),
        5 => prop::collection::vec(read_header(), 1..4).prop_map(Op::Read),
        4 => confirm_choice().prop_map(Op::SolicitedConfirm),
        4 => confirm_choice().prop_map(Op::UnsolicitedConfirm),
        2 => classes().prop_map(Op::Enable),
        1 => classes().prop_map(Op::Disable),
        1 => Just(Op::DelayMeasure),
        1 => Just(Op::ClearRestart),
        2 => Just(Op::Repeat),
        4 => advance_choice().prop_map(Op::Advance),
        2 => Just(Op::Reconnect),
    ]
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
    let mut config = get_default_unsolicited_config();
    config.features.unsolicited = if setup.unsolicited {
        Feature::Enabled
    } else {
        Feature::Disabled
    };
    config.keep_alive_timeout = None;
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

/// Run a test on a current-thread runtime with the clock paused, under a [`watchdog`]
fn run_paused(test: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap()
        .block_on(watchdog(test))
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
        Input::Confirm { uns, seq } => {
            harness
                .send_and_process(&Input::confirm_bytes(*uns, *seq))
                .await
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
    let mut writes = Vec::new();
    while let Some(bytes) = harness.pop_response() {
        writes.push(bytes);
    }
    let mut callbacks = Vec::new();
    while let Some(event) = harness.poll_event() {
        callbacks.push(event);
    }

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
    // deletes at least one event but the first: it may confirm the null response, or a response
    // whose events were all evicted before the drain. The DISABLE and the READ make two more inputs.
    let limit = model.events.len() + 3;
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
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn session_follows_the_event_and_confirm_model(
        setup in setup(),
        ops in prop::collection::vec(op(), 1..48),
    ) {
        run_paused(check_model(setup, ops));
    }
}
