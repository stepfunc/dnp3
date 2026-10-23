// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! One task against a generated stream of answers and noise, checked step by step against a model
//! of what the master writes, reports and delivers, and how the task ends
//!
//! Every input is followed by a wait on the paused clock, which lets the master process it. Waits
//! are multiples of [`TICK`] and the response timeout isn't, so no input arrives at the instant a
//! deadline expires. Once the task ends, a read requested through the API shows which sequence
//! number the association continues from.

use std::time::Duration;

use proptest::prelude::*;
use proptest::test_runner::TestCaseError;
use sfio_tokio_mock_io::Event;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::app::{FunctionCode, Iin, Iin1, Iin2, ObjectParseError, RetryStrategy, Sequence};
use crate::link::EndpointAddress;
use crate::master::association::AssociationConfig;
use crate::master::test_fixtures::FixedTime;
use crate::master::tests::harness::{
    create_association_with_handler, AssocInfoEvent, ReadEvent, TestHarness,
};
use crate::master::tests::outcomes::{
    Entry, Kind, Log, Reply, MALFORMED, OTHER, OUTSTATION, UNKNOWN,
};
use crate::master::{Classes, CommandError, CommandResponseError, ReadType, TaskError, TaskType};
use crate::util::test_support::run_paused;

/// The response timeout the harness configures, in milliseconds
const TIMEOUT: u64 = 1000;
/// Every wait is a multiple of this many milliseconds, and [`TIMEOUT`] isn't
const TICK: u64 = 7;
/// A wait longer than a response timeout, which ends any task still waiting
const FINAL_WAIT: u64 = 143 * TICK;

const RESTART: u8 = 0x80;
const BAD_REQUEST_ERRORS: u8 = 0x07;

/// The task under test
#[derive(Copy, Clone, Debug)]
enum Task {
    /// a task requested through the API
    User(Kind),
    /// the automatic integrity scan at startup
    StartupIntegrity,
}

impl Task {
    fn task_type(self) -> TaskType {
        match self {
            Task::User(kind) => kind.task_type(),
            Task::StartupIntegrity => TaskType::StartupIntegrity,
        }
    }

    fn function(self) -> FunctionCode {
        match self {
            Task::User(kind) => kind.function(),
            Task::StartupIntegrity => FunctionCode::Read,
        }
    }

    fn is_read(self) -> bool {
        matches!(self, Task::User(Kind::Read) | Task::StartupIntegrity)
    }

    fn read_type(self) -> ReadType {
        match self {
            Task::StartupIntegrity => ReadType::StartupIntegrity,
            Task::User(_) => ReadType::SinglePoll,
        }
    }

    /// A read the user asked for fails if the outstation reported a request error and returned no
    /// values; the startup integrity scan completes anyway
    fn fails_without_data(self) -> bool {
        matches!(self, Task::User(_))
    }

    /// Unsolicited responses with data are accepted only after the startup integrity scan
    fn accepts_unsolicited_data(self) -> bool {
        matches!(self, Task::User(_))
    }

    /// True if a correct response is a control echo, and a wrong one fails as a command
    fn is_command(self) -> bool {
        matches!(
            self,
            Task::User(Kind::DirectOperate | Kind::SelectBeforeOperate)
        )
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Source {
    Outstation,
    /// another outstation, with its own association
    Other,
    /// an address without an association
    Unknown,
}

impl Source {
    fn address(self) -> EndpointAddress {
        match self {
            Source::Outstation => OUTSTATION,
            Source::Other => OTHER,
            Source::Unknown => UNKNOWN,
        }
    }

    fn index(self) -> usize {
        match self {
            Source::Outstation => 0,
            Source::Other => 1,
            Source::Unknown => 2,
        }
    }
}

/// The objects of a fragment. Reads and unsolicited responses use the measurement kinds; other
/// tasks use `Correct`, `Unexpected` and `Malformed`.
#[derive(Copy, Clone, Debug, PartialEq)]
enum Objects {
    None,
    /// one analog input event (g32v2)
    AnalogEvent,
    /// one static analog input (g30v2)
    AnalogStatic,
    /// a common time of occurrence, then a binary input event with relative time
    CtoRelative,
    /// a binary input event with relative time and no common time of occurrence before it
    RelativeWithoutCto,
    /// an object header cut short after its group
    Malformed,
    /// the objects the task expects
    Correct,
    /// objects that parse, but that the task doesn't expect
    Unexpected,
}

/// What a read or unsolicited response delivers, and how the handler sees it
#[derive(Copy, Clone, Debug, PartialEq)]
enum Measurement {
    Analog,
    Binary,
}

/// What the master can make of a read or unsolicited fragment's objects
#[derive(Copy, Clone, Debug, PartialEq)]
enum Content {
    Valid(Option<Measurement>),
    Malformed,
    RelativeWithoutCto,
}

impl Objects {
    fn content(self) -> Content {
        match self {
            Objects::None | Objects::Correct | Objects::Unexpected => Content::Valid(None),
            Objects::AnalogEvent | Objects::AnalogStatic => {
                Content::Valid(Some(Measurement::Analog))
            }
            Objects::CtoRelative => Content::Valid(Some(Measurement::Binary)),
            Objects::RelativeWithoutCto => Content::RelativeWithoutCto,
            Objects::Malformed => Content::Malformed,
        }
    }

    fn measurement_bytes(self) -> Vec<u8> {
        match self {
            Objects::None | Objects::Correct | Objects::Unexpected => vec![],
            Objects::AnalogEvent => vec![32, 2, 0x17, 1, 0, 0x01, 7, 0],
            Objects::AnalogStatic => vec![30, 2, 0x00, 0, 0, 0x01, 7, 0],
            Objects::CtoRelative => vec![
                51, 1, 0x07, 1, 0, 0, 0, 0, 0, 0, 2, 3, 0x17, 1, 0, 0x81, 0x10, 0x00,
            ],
            Objects::RelativeWithoutCto => vec![2, 3, 0x17, 1, 0, 0x81, 0x10, 0x00],
            Objects::Malformed => vec![0x01],
        }
    }
}

/// What a non-read task can make of a response's objects
#[derive(Copy, Clone, Debug, PartialEq)]
enum Reaction {
    Correct,
    Unexpected,
    Malformed,
}

impl Reaction {
    fn of(objects: Objects) -> Self {
        match objects {
            Objects::Correct => Reaction::Correct,
            Objects::Malformed => Reaction::Malformed,
            Objects::None
            | Objects::AnalogEvent
            | Objects::AnalogStatic
            | Objects::CtoRelative
            | Objects::RelativeWithoutCto
            | Objects::Unexpected => Reaction::Unexpected,
        }
    }

    fn bytes(self, kind: Kind, request: &[u8]) -> Vec<u8> {
        match self {
            Reaction::Correct => kind.first_response(request).split_off(4),
            // a task that expects objects gets none, and one that expects none gets a time delay
            Reaction::Unexpected => match kind {
                Kind::EmptyResponse | Kind::DeadBands | Kind::TimeSync => {
                    vec![52, 1, 0x07, 1, 0x02, 0x00]
                }
                _ => vec![],
            },
            Reaction::Malformed => vec![0x01],
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum SeqChoice {
    /// the sequence number the master expects
    Expected,
    /// the one before it, as a repeat of the previous fragment has
    Previous,
    /// the one after it
    Next,
}

#[derive(Copy, Clone, Debug)]
enum Fir {
    /// set on the first fragment of a response only
    Proper,
    Flipped,
}

/// A solicited response fragment for the task's current request
#[derive(Copy, Clone, Debug)]
struct Answer {
    source: Source,
    seq: SeqChoice,
    fir: Fir,
    fin: bool,
    con: bool,
    restart: bool,
    iin2: u8,
    objects: Objects,
}

#[derive(Copy, Clone, Debug)]
struct Unsolicited {
    source: Source,
    seq: u8,
    con: bool,
    restart: bool,
    objects: Objects,
    /// send the last unsolicited fragment from this source again, if there is one
    repeat: bool,
}

impl Unsolicited {
    fn bytes(self) -> Vec<u8> {
        let iin1 = if self.restart { RESTART } else { 0 };
        fragment(
            true,
            true,
            self.con,
            true,
            self.seq,
            iin1,
            0,
            &self.objects.measurement_bytes(),
        )
    }
}

#[derive(Copy, Clone, Debug)]
enum Input {
    Answer(Answer),
    Unsolicited(Unsolicited),
    /// a fragment that isn't a valid response, one of [`MALFORMED`]
    Malformed(Source, usize),
    /// a request through the API that doesn't involve the outstation
    Api,
    Silence,
}

#[derive(Copy, Clone, Debug)]
struct Step {
    input: Input,
    /// milliseconds to wait after the input
    wait: u64,
}

#[expect(clippy::too_many_arguments, reason = "every field of the header")]
fn fragment(
    fir: bool,
    fin: bool,
    con: bool,
    uns: bool,
    seq: u8,
    iin1: u8,
    iin2: u8,
    objects: &[u8],
) -> Vec<u8> {
    let control = (u8::from(fir) << 7)
        | (u8::from(fin) << 6)
        | (u8::from(con) << 5)
        | (u8::from(uns) << 4)
        | (seq & 0x0F);
    let function = if uns { 0x82 } else { 0x81 };
    let mut bytes = vec![control, function, iin1, iin2];
    bytes.extend_from_slice(objects);
    bytes
}

fn next_seq(seq: u8) -> u8 {
    seq.wrapping_add(1) & 0x0F
}

fn previous_seq(seq: u8) -> u8 {
    seq.wrapping_add(15) & 0x0F
}

/// What the master wrote
#[derive(Clone, Debug, PartialEq)]
enum Write {
    /// a request with FIR and FIN, its function code and sequence number
    Request(FunctionCode, u8),
    /// a solicited confirm, and its sequence number
    Confirm(u8),
    /// an unsolicited confirm, and its sequence number
    UnsolicitedConfirm(u8),
    Other(Vec<u8>),
}

impl Write {
    fn classify(bytes: &[u8]) -> Self {
        if let [control, function, rest @ ..] = bytes {
            let seq = control & 0x0F;
            match (control & 0xF0, FunctionCode::from(*function)) {
                (0xC0, Some(FunctionCode::Confirm)) if rest.is_empty() => {
                    return Write::Confirm(seq)
                }
                (0xD0, Some(FunctionCode::Confirm)) if rest.is_empty() => {
                    return Write::UnsolicitedConfirm(seq)
                }
                (0xC0, Some(function)) if function != FunctionCode::Confirm => {
                    return Write::Request(function, seq)
                }
                _ => {}
            }
        }
        Write::Other(bytes.to_vec())
    }
}

/// Everything observable after one step
#[derive(Clone, Debug, Default, PartialEq)]
struct StepObs {
    writes: Vec<Write>,
    /// how many fragments the master read
    reads: usize,
    /// read handler callbacks of the association under test
    main: Vec<ReadEvent>,
    /// association information callbacks of the association under test
    info: Vec<AssocInfoEvent>,
    /// callbacks of the other outstation's association
    other: Vec<Entry>,
}

/// How the task ended
#[derive(Copy, Clone, Debug, PartialEq)]
enum End {
    /// with the sequence number of the last response
    Success(u8),
    Failure(TaskError),
    /// a command whose response doesn't echo the request
    CommandMismatch,
}

impl End {
    fn event(self, task: Task) -> AssocInfoEvent {
        match self {
            End::Success(seq) => {
                AssocInfoEvent::TaskSuccess(task.task_type(), task.function(), Sequence::new(seq))
            }
            End::Failure(err) => AssocInfoEvent::TaskFailure(task.task_type(), err),
            End::CommandMismatch => {
                AssocInfoEvent::TaskFailure(task.task_type(), TaskError::UnexpectedResponseHeaders)
            }
        }
    }

    fn reply(self, kind: Kind) -> Reply {
        match self {
            End::Success(_) => kind.ok(),
            End::Failure(err) => kind.failed(err),
            End::CommandMismatch => Reply::Command(Err(CommandError::Response(
                CommandResponseError::HeaderCountMismatch,
            ))),
        }
    }
}

/// How and when the task ended, and what a read requested next through the API then writes
#[derive(Clone, Debug)]
struct Ending {
    end: End,
    time: u64,
    probe: StepObs,
}

/// One allowed outcome of a step: what's observed, and how the task ended in it, if it did
#[derive(Clone, Debug)]
struct Allowed {
    obs: StepObs,
    ending: Option<Ending>,
}

enum State {
    Read {
        first: bool,
        seq: u8,
        deadline: u64,
        /// IIN2 request errors of every fragment
        errors: u8,
        /// IIN1 of the last fragment
        iin1: u8,
        values: bool,
    },
    Response {
        /// the select of a select-before-operate was answered
        operating: bool,
        seq: u8,
        deadline: u64,
    },
    Done,
}

/// What the master should do, written from the rules rather than the code
struct Model {
    task: Task,
    now: u64,
    /// the sequence number of the association's next request
    next_seq: u8,
    state: State,
    /// a fragment the master processed reported the restart IIN bit, even if the master then
    /// rejected the fragment. Fragments it ignores don't count: another outstation's, a response
    /// to another request, or a solicited fragment that breaks the FIR, FIN or CON rules
    restart: bool,
    /// the last unsolicited fragment accepted from the outstation and from the other one
    accepted: [Option<Vec<u8>>; 2],
    /// the last unsolicited fragment sent from each source
    sent: [Option<Unsolicited>; 3],
    obs: StepObs,
    /// the ways the task may have ended in this step, and when
    ends: Vec<End>,
    end_time: u64,
}

impl Model {
    fn new(task: Task) -> Self {
        let state = if task.is_read() {
            State::Read {
                first: true,
                seq: 0,
                deadline: TIMEOUT,
                errors: 0,
                iin1: 0,
                values: false,
            }
        } else {
            State::Response {
                operating: false,
                seq: 0,
                deadline: TIMEOUT,
            }
        };
        let obs = StepObs {
            writes: vec![Write::Request(task.function(), 0)],
            info: vec![AssocInfoEvent::TaskStart(
                task.task_type(),
                task.function(),
                Sequence::new(0),
            )],
            ..StepObs::default()
        };
        Self {
            task,
            now: 0,
            next_seq: 1,
            state,
            restart: false,
            accepted: [None, None],
            sent: [None, None, None],
            obs,
            ends: vec![],
            end_time: 0,
        }
    }

    fn done(&self) -> bool {
        matches!(self.state, State::Done)
    }

    fn deadline(&self) -> Option<u64> {
        match self.state {
            State::Read { deadline, .. } | State::Response { deadline, .. } => Some(deadline),
            State::Done => None,
        }
    }

    fn saw_restart(&mut self, restart: bool) {
        self.restart |= restart;
    }

    fn end(&mut self, ends: Vec<End>) {
        self.state = State::Done;
        self.ends = ends;
        self.end_time = self.now;
    }

    /// How long to wait after an input: the step's wait, but no more than a short while after the
    /// task ends, so that nothing the end leads to, such as clearing a restart, times out in the step
    fn wait(&self, wait: u64) -> u64 {
        match self.deadline() {
            None => TICK,
            Some(deadline) => wait.min(deadline.saturating_sub(self.now).saturating_add(TICK)),
        }
    }

    /// If a wait reaches the deadline, the part of it that ends a millisecond before
    fn before_deadline(&self, wait: u64) -> Option<u64> {
        let remaining = self.deadline()?.saturating_sub(self.now);
        (remaining <= wait).then(|| remaining.saturating_sub(1))
    }

    /// Wait, ending the task if its deadline passes
    fn advance(&mut self, wait: u64) {
        let until = self.now.saturating_add(wait);
        if let Some(deadline) = self.deadline() {
            if deadline <= until {
                self.now = deadline;
                self.end(vec![End::Failure(TaskError::ResponseTimeout)]);
            }
        }
        self.now = until;
    }

    /// The allowed outcomes of the step that just ended
    fn take_step(&mut self) -> Vec<Allowed> {
        let obs = std::mem::take(&mut self.obs);
        let ends = std::mem::take(&mut self.ends);
        if ends.is_empty() {
            return vec![Allowed { obs, ending: None }];
        }
        // the restart is cleared once the task ends
        let clear = self.restart;
        // the next request continues from the task's last sequence number
        let (task_type, function) = if clear {
            (TaskType::ClearRestartBit, FunctionCode::Write)
        } else {
            (TaskType::UserRead, FunctionCode::Read)
        };
        let mut allowed = vec![];
        for end in ends {
            let mut obs = obs.clone();
            obs.info.push(end.event(self.task));
            let next = StepObs {
                writes: vec![Write::Request(function, self.next_seq)],
                info: vec![AssocInfoEvent::TaskStart(
                    task_type,
                    function,
                    Sequence::new(self.next_seq),
                )],
                ..StepObs::default()
            };
            // clearing the restart starts at once, and the read waits for its response
            let probe = if clear {
                obs.writes.extend(next.writes);
                obs.info.extend(next.info);
                StepObs::default()
            } else {
                next
            };
            allowed.push(Allowed {
                obs,
                ending: Some(Ending {
                    end,
                    time: self.end_time,
                    probe,
                }),
            });
        }
        allowed
    }

    /// Apply an input, returning the fragment to send and where it comes from. `request` is the
    /// last request the master wrote, which a control response echoes.
    fn input(&mut self, input: Input, request: &[u8]) -> Option<(EndpointAddress, Vec<u8>)> {
        match input {
            Input::Silence | Input::Api => None,
            Input::Malformed(source, which) => {
                self.obs.reads = 1;
                if source == Source::Outstation {
                    self.end(vec![End::Failure(TaskError::Transport)]);
                }
                let bytes = MALFORMED.get(which).copied().unwrap_or_default();
                Some((source.address(), bytes.to_vec()))
            }
            Input::Unsolicited(unsolicited) => {
                let mut unsolicited = match self.sent[unsolicited.source.index()] {
                    Some(last) if unsolicited.repeat => last,
                    _ => unsolicited,
                };
                if unsolicited.source != Source::Outstation {
                    unsolicited.restart = false;
                }
                self.sent[unsolicited.source.index()] = Some(unsolicited);
                self.obs.reads = 1;
                let bytes = unsolicited.bytes();
                self.unsolicited(unsolicited, &bytes);
                Some((unsolicited.source.address(), bytes))
            }
            Input::Answer(answer) => {
                self.obs.reads = 1;
                let bytes = self.answer(answer, request);
                Some((answer.source.address(), bytes))
            }
        }
    }

    fn unsolicited(&mut self, unsolicited: Unsolicited, bytes: &[u8]) {
        let accepted = match unsolicited.source {
            Source::Unknown => return,
            Source::Outstation => &self.accepted[0],
            Source::Other => &self.accepted[1],
        };
        let restart = unsolicited.restart;
        let seq = Sequence::new(unsolicited.seq);
        let content = unsolicited.objects.content();
        let null = unsolicited.objects.measurement_bytes().is_empty();
        let is_duplicate = accepted.as_deref() == Some(bytes);

        // the IIN of an unsolicited response from an association is acted on even if the response
        // is a duplicate, comes before the startup integrity scan, or has objects it can't process
        self.saw_restart(restart);
        if unsolicited.source == Source::Outstation
            && !(self.task.accepts_unsolicited_data() || null)
        {
            return;
        }
        if is_duplicate {
            self.record_unsolicited(unsolicited.source, AssocInfoEvent::Unsolicited(true, seq));
            if unsolicited.con {
                self.obs
                    .writes
                    .push(Write::UnsolicitedConfirm(unsolicited.seq));
            }
            return;
        }
        let Content::Valid(measurement) = content else {
            // neither processed nor confirmed, and not remembered
            return;
        };
        match unsolicited.source {
            Source::Outstation => {
                self.accepted[0] = Some(bytes.to_vec());
                self.obs
                    .main
                    .extend(delivery(ReadType::Unsolicited, measurement));
            }
            Source::Other => {
                self.accepted[1] = Some(bytes.to_vec());
                self.obs.other.extend(
                    delivery(ReadType::Unsolicited, measurement)
                        .into_iter()
                        .map(Entry::Read),
                );
            }
            Source::Unknown => {}
        }
        self.record_unsolicited(unsolicited.source, AssocInfoEvent::Unsolicited(false, seq));
        if unsolicited.con {
            self.obs
                .writes
                .push(Write::UnsolicitedConfirm(unsolicited.seq));
        }
    }

    fn record_unsolicited(&mut self, source: Source, event: AssocInfoEvent) {
        match source {
            Source::Outstation => self.obs.info.push(event),
            Source::Other => self.obs.other.push(Entry::Info(event)),
            Source::Unknown => {}
        }
    }

    fn answer(&mut self, answer: Answer, request: &[u8]) -> Vec<u8> {
        let (expected, proper_fir) = match self.state {
            State::Read { first, seq, .. } => (seq, first),
            State::Response { seq, .. } => (seq, true),
            State::Done => (self.next_seq, true),
        };
        let seq = match answer.seq {
            SeqChoice::Expected => expected,
            SeqChoice::Previous => previous_seq(expected),
            SeqChoice::Next => next_seq(expected),
        };
        let fir = match answer.fir {
            Fir::Proper => proper_fir,
            Fir::Flipped => !proper_fir,
        };
        let iin1 = if answer.restart { RESTART } else { 0 };
        let objects = match self.task {
            Task::User(kind) if !self.task.is_read() => {
                Reaction::of(answer.objects).bytes(kind, request)
            }
            Task::User(_) | Task::StartupIntegrity => answer.objects.measurement_bytes(),
        };
        let bytes = fragment(
            fir,
            answer.fin,
            answer.con,
            false,
            seq,
            iin1,
            answer.iin2,
            &objects,
        );

        // responses from other addresses are no concern of this task
        if answer.source != Source::Outstation {
            return bytes;
        }
        // nor are responses to other requests, whose IIN may be stale
        if seq != expected {
            return bytes;
        }
        if self.task.is_read() {
            self.read_fragment(answer, seq, fir, iin1);
        } else {
            self.response(answer, seq, fir, iin1);
        }
        bytes
    }

    fn read_fragment(&mut self, answer: Answer, seq: u8, fir: bool, iin1: u8) {
        let State::Read { first, .. } = self.state else {
            return;
        };
        // any rule the fragment breaks may be the one reported
        let mut broken = vec![];
        if fir && !first {
            broken.push(End::Failure(TaskError::UnexpectedFir));
        }
        if !fir && first {
            broken.push(End::Failure(TaskError::NeverReceivedFir));
        }
        if !answer.fin && !answer.con {
            broken.push(End::Failure(TaskError::NonFinWithoutCon));
        }
        if !broken.is_empty() {
            self.end(broken);
            return;
        }

        // the IIN of the response is acted on even if its objects are rejected
        self.saw_restart(answer.restart);
        let measurement = match answer.objects.content() {
            Content::Valid(measurement) => measurement,
            Content::Malformed => {
                self.end(vec![End::Failure(TaskError::MalformedResponse(
                    ObjectParseError::InsufficientBytes,
                ))]);
                return;
            }
            Content::RelativeWithoutCto => {
                self.end(vec![End::Failure(TaskError::UnexpectedResponseHeaders)]);
                return;
            }
        };

        self.obs
            .main
            .extend(delivery(self.task.read_type(), measurement));
        if answer.con {
            self.obs.writes.push(Write::Confirm(seq));
        }

        let fails_without_data = self.task.fails_without_data();
        let next = self.next_seq;
        let now = self.now;
        let State::Read {
            first,
            seq: expected,
            deadline,
            errors,
            iin1: last_iin1,
            values,
        } = &mut self.state
        else {
            return;
        };
        *errors |= answer.iin2 & BAD_REQUEST_ERRORS;
        *last_iin1 = iin1;
        *values |= measurement.is_some();

        if answer.fin {
            let end = if *errors == 0 || *values || !fails_without_data {
                End::Success(seq)
            } else {
                End::Failure(TaskError::RejectedByIin2(Iin::new(
                    Iin1::new(*last_iin1),
                    Iin2::new(*errors),
                )))
            };
            self.end(vec![end]);
        } else {
            // the next fragment has the next sequence number, and its own response timeout
            *first = false;
            *expected = next;
            *deadline = now.saturating_add(TIMEOUT);
            self.next_seq = next_seq(next);
        }
    }

    fn response(&mut self, answer: Answer, seq: u8, fir: bool, iin1: u8) {
        let State::Response { operating, .. } = self.state else {
            return;
        };
        if !(fir && answer.fin) {
            self.end(vec![End::Failure(TaskError::MultiFragmentResponse)]);
            return;
        }

        let reaction = Reaction::of(answer.objects);
        let malformed = reaction == Reaction::Malformed;
        // a response whose objects parse is confirmed before anything else, even if it's rejected
        if answer.con && !malformed {
            self.obs.writes.push(Write::Confirm(seq));
        }
        // the IIN of the response is acted on even if the response is rejected
        self.saw_restart(answer.restart);

        let mut ends = vec![];
        let rejected = answer.iin2 & BAD_REQUEST_ERRORS != 0;
        if rejected {
            ends.push(End::Failure(TaskError::RejectedByIin2(Iin::new(
                Iin1::new(iin1),
                Iin2::new(answer.iin2),
            ))));
        }
        // a rejected response is reported as such, unless its objects can't be parsed either
        if !rejected || malformed {
            match reaction {
                Reaction::Malformed => ends.push(End::Failure(TaskError::MalformedResponse(
                    ObjectParseError::InsufficientBytes,
                ))),
                Reaction::Unexpected if self.task.is_command() => ends.push(End::CommandMismatch),
                Reaction::Unexpected => {
                    ends.push(End::Failure(TaskError::UnexpectedResponseHeaders))
                }
                Reaction::Correct => {
                    let selected =
                        matches!(self.task, Task::User(Kind::SelectBeforeOperate)) && !operating;
                    if selected {
                        // then the operate, with the next sequence number
                        let seq = self.next_seq;
                        self.obs
                            .writes
                            .push(Write::Request(FunctionCode::Operate, seq));
                        self.state = State::Response {
                            operating: true,
                            seq,
                            deadline: self.now.saturating_add(TIMEOUT),
                        };
                        self.next_seq = next_seq(seq);
                        return;
                    }
                    ends.push(End::Success(seq));
                }
            }
        }
        self.end(ends);
    }
}

/// The read handler callbacks for one fragment
fn delivery(read_type: ReadType, measurement: Option<Measurement>) -> Vec<ReadEvent> {
    let mut events = vec![ReadEvent::BeginFragment(read_type)];
    match measurement {
        Some(Measurement::Analog) => events.push(ReadEvent::AnalogInput),
        Some(Measurement::Binary) => events.push(ReadEvent::BinaryInput),
        None => {}
    }
    events.push(ReadEvent::EndFragment(read_type));
    events
}

/// What the requester got, and when
#[derive(Debug, PartialEq)]
enum ReplyObs {
    /// the task has no requester
    None,
    /// no reply yet
    Pending,
    /// the reply, and the milliseconds since the task started
    Done(Reply, u64),
}

/// A case run against the master, with the model's allowed outcomes for every step
struct Run {
    task: Task,
    allowed: Vec<Vec<Allowed>>,
    observed: Vec<StepObs>,
    reply: ReplyObs,
    /// what the master did after a read was requested through the API, once the task ended
    probe: StepObs,
}

impl Run {
    fn check(&self) -> Result<(), TestCaseError> {
        prop_assert_eq!(self.allowed.len(), self.observed.len());
        let mut ending = None;
        for (step, (allowed, observed)) in self.allowed.iter().zip(&self.observed).enumerate() {
            let matched = allowed.iter().find(|allowed| allowed.obs == *observed);
            let Some(matched) = matched else {
                return Err(TestCaseError::fail(format!(
                    "step {step}: observed {observed:#?}\nallowed {allowed:#?}"
                )));
            };
            if let Some(matched) = &matched.ending {
                prop_assert!(ending.is_none(), "step {}: the task ended twice", step);
                ending = Some(matched);
            }
        }
        let Some(ending) = ending else {
            return Err(TestCaseError::fail("the task never ended"));
        };
        let expected = match self.task {
            Task::User(kind) => ReplyObs::Done(ending.end.reply(kind), ending.time),
            Task::StartupIntegrity => ReplyObs::None,
        };
        prop_assert_eq!(&self.reply, &expected);
        prop_assert_eq!(&self.probe, &ending.probe);
        Ok(())
    }
}

async fn start(task: Task) -> (TestHarness, Log) {
    let config = match task {
        Task::User(_) => AssociationConfig::quiet(),
        Task::StartupIntegrity => AssociationConfig {
            startup_integrity_classes: Classes::class0(),
            // a failed scan isn't retried while the case runs
            auto_tasks_retry_strategy: RetryStrategy::new(
                Duration::from_secs(3600),
                Duration::from_secs(3600),
            ),
            ..AssociationConfig::quiet()
        },
    };
    let mut harness = create_association_with_handler(config, Box::new(FixedTime)).await;
    let other = Log::default();
    harness
        .master
        .add_association(
            OTHER,
            AssociationConfig::quiet(),
            Box::new(other.clone()),
            Box::new(other.clone()),
            Box::new(other.clone()),
        )
        .await
        .unwrap();
    (harness, other)
}

fn observe(harness: &mut TestHarness, other: &Log, last_request: &mut Vec<u8>) -> StepObs {
    let mut obs = StepObs::default();
    while let Some(event) = harness.io.pop_event() {
        match event {
            Event::Write(bytes) => {
                let write = Write::classify(&bytes);
                if matches!(write, Write::Request(..)) {
                    *last_request = bytes;
                }
                obs.writes.push(write);
            }
            Event::Read => obs.reads = obs.reads.saturating_add(1),
            Event::WriteErr | Event::ReadErr => obs.writes.push(Write::Other(vec![])),
        }
    }
    obs.main = harness.read_events.pop();
    obs.info = harness.assoc_events.pop();
    obs.other = other.pop();
    obs
}

async fn wait(ms: u64) {
    tokio::time::sleep(Duration::from_millis(ms)).await;
}

/// Run the task against the steps, stopping once the model says it ended, and silence after them
async fn execute(task: Task, steps: &[Step]) -> Run {
    let (mut harness, other) = start(task).await;
    let started = Instant::now();
    let reply: Option<JoinHandle<(Reply, Instant)>> = match task {
        Task::User(kind) => {
            let association = harness.association.clone();
            Some(tokio::spawn(async move {
                let reply = kind.spawn(association).await.unwrap();
                (reply, Instant::now())
            }))
        }
        Task::StartupIntegrity => None,
    };

    let mut model = Model::new(task);
    let mut last_request = vec![];
    let mut allowed = vec![];
    let mut observed = vec![];

    model.advance(TICK);
    wait(TICK).await;
    allowed.push(model.take_step());
    observed.push(observe(&mut harness, &other, &mut last_request));

    let silence = Step {
        input: Input::Silence,
        wait: FINAL_WAIT,
    };
    for step in steps.iter().chain([&silence]) {
        if model.done() {
            break;
        }
        if let Input::Api = step.input {
            let mut master = harness.master.clone();
            drop(tokio::spawn(async move { master.get_decode_level().await }));
        }
        if let Some((source, bytes)) = model.input(step.input, &last_request) {
            harness.set_source(source);
            harness.io.read(&bytes);
        }
        let mut ms = model.wait(step.wait);
        // a wait through the deadline is also observed just before it, so a task that times out
        // early is seen, whether or not it has a requester whose reply is timed
        if let Some(before) = model.before_deadline(ms) {
            model.advance(before);
            wait(before).await;
            allowed.push(model.take_step());
            observed.push(observe(&mut harness, &other, &mut last_request));
            ms = ms.saturating_sub(before);
        }
        model.advance(ms);
        wait(ms).await;
        allowed.push(model.take_step());
        observed.push(observe(&mut harness, &other, &mut last_request));
    }

    // a read requested now is the next request, unless clearing the restart is
    drop(Kind::Read.spawn(harness.association.clone()));
    wait(TICK).await;
    let probe = observe(&mut harness, &other, &mut last_request);

    let reply = match reply {
        None => ReplyObs::None,
        Some(reply) if !reply.is_finished() => ReplyObs::Pending,
        Some(reply) => {
            let (reply, at) = reply.await.unwrap();
            let elapsed = at.duration_since(started).as_millis();
            ReplyObs::Done(reply, u64::try_from(elapsed).unwrap())
        }
    };
    Run {
        task,
        allowed,
        observed,
        reply,
        probe,
    }
}

fn source() -> impl Strategy<Value = Source> {
    prop_oneof![
        8 => Just(Source::Outstation),
        1 => Just(Source::Other),
        1 => Just(Source::Unknown),
    ]
}

/// mostly short waits, sometimes a large part of a response timeout, and rarely one just short of
/// it or past it
fn wait_ms() -> impl Strategy<Value = u64> {
    prop_oneof![
        16 => Just(1u64),
        4 => Just(2),
        2 => Just(35),
        2 => Just(71),
        1 => Just(142),
        1 => Just(143),
    ]
    .prop_map(|ticks| ticks.saturating_mul(TICK))
}

/// mostly no IIN2 bits, then each request error, and the event buffer overflow that isn't one
fn iin2() -> impl Strategy<Value = u8> {
    prop_oneof![
        6 => Just(0u8),
        1 => Just(0x01),
        1 => Just(0x02),
        1 => Just(0x04),
        1 => Just(0x08),
    ]
}

fn measurement_objects() -> impl Strategy<Value = Objects> {
    prop_oneof![
        4 => Just(Objects::None),
        3 => Just(Objects::AnalogEvent),
        2 => Just(Objects::AnalogStatic),
        1 => Just(Objects::CtoRelative),
        1 => Just(Objects::RelativeWithoutCto),
        1 => Just(Objects::Malformed),
    ]
}

fn response_objects() -> impl Strategy<Value = Objects> {
    prop_oneof![
        6 => Just(Objects::Correct),
        1 => Just(Objects::Unexpected),
        1 => Just(Objects::Malformed),
    ]
}

fn answer(read: bool) -> impl Strategy<Value = Answer> {
    let seq = prop_oneof![
        8 => Just(SeqChoice::Expected),
        1 => Just(SeqChoice::Previous),
        1 => Just(SeqChoice::Next),
    ];
    let fir = prop_oneof![10 => Just(Fir::Proper), 1 => Just(Fir::Flipped)];
    // a read's response mostly continues in another fragment, as a correct one must with CON
    let fin_con = if read {
        prop_oneof![
            6 => Just((false, true)),
            3 => Just((true, false)),
            2 => Just((true, true)),
            1 => Just((false, false)),
        ]
        .boxed()
    } else {
        prop_oneof![
            8 => Just((true, false)),
            4 => Just((true, true)),
            1 => Just((false, true)),
            1 => Just((false, false)),
        ]
        .boxed()
    };
    let objects = if read {
        measurement_objects().boxed()
    } else {
        response_objects().boxed()
    };
    (
        source(),
        seq,
        fir,
        fin_con,
        prop::bool::weighted(0.15),
        iin2(),
        objects,
    )
        .prop_map(
            |(source, seq, fir, (fin, con), restart, iin2, objects)| Answer {
                source,
                seq,
                fir,
                fin,
                con,
                restart,
                iin2,
                objects,
            },
        )
}

fn unsolicited() -> impl Strategy<Value = Unsolicited> {
    let source = prop_oneof![
        3 => Just(Source::Outstation),
        1 => Just(Source::Other),
        1 => Just(Source::Unknown),
    ];
    (
        source,
        0u8..16,
        any::<bool>(),
        prop::bool::weighted(0.15),
        measurement_objects(),
        prop::bool::weighted(0.3),
    )
        .prop_map(|(source, seq, con, restart, objects, repeat)| Unsolicited {
            source,
            seq,
            con,
            restart,
            objects,
            repeat,
        })
}

fn input(read: bool) -> impl Strategy<Value = Input> {
    // a malformed fragment from the outstation ends the task, so it comes from others more often
    let malformed_source = prop_oneof![
        1 => Just(Source::Outstation),
        2 => Just(Source::Other),
        2 => Just(Source::Unknown),
    ];
    prop_oneof![
        10 => answer(read).prop_map(Input::Answer),
        3 => unsolicited().prop_map(Input::Unsolicited),
        1 => (malformed_source, 0..MALFORMED.len()).prop_map(|(source, which)| Input::Malformed(source, which)),
        1 => Just(Input::Api),
        1 => Just(Input::Silence),
    ]
}

fn steps(read: bool) -> impl Strategy<Value = Vec<Step>> {
    prop::collection::vec(
        (input(read), wait_ms()).prop_map(|(input, wait)| Step { input, wait }),
        1..=10,
    )
}

proptest! {
    // more cases than the other tasks, since few of them read several fragments
    #![proptest_config(ProptestConfig::with_cases(128))]

    #[test]
    fn read_tasks_match_the_model(
        task in prop_oneof![Just(Task::User(Kind::Read)), Just(Task::StartupIntegrity)],
        steps in steps(true),
    ) {
        run_paused(execute(task, &steps)).check()?;
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn non_read_tasks_match_the_model(
        kind in prop::sample::select(vec![
            Kind::DirectOperate,
            Kind::SelectBeforeOperate,
            Kind::ColdRestart,
            Kind::EmptyResponse,
            Kind::DeadBands,
            Kind::AuthFile,
            Kind::OpenFile,
            Kind::CloseFile,
            Kind::WriteFileBlock,
            Kind::GetFileInfo,
        ]),
        steps in steps(false),
    ) {
        run_paused(execute(Task::User(kind), &steps)).check()?;
    }
}

/// A read whose first fragment reports an unknown object but has data, and asks for a confirm,
/// then a last fragment without data
fn read_with_a_rejected_fragment() -> Vec<Step> {
    let fragment = |fin, iin2, objects| Step {
        input: Input::Answer(Answer {
            source: Source::Outstation,
            seq: SeqChoice::Expected,
            fir: Fir::Proper,
            fin,
            con: true,
            restart: false,
            iin2,
            objects,
        }),
        wait: TICK,
    };
    vec![
        fragment(false, 0x02, Objects::AnalogEvent),
        fragment(true, 0x00, Objects::None),
    ]
}

#[test]
fn the_model_accepts_the_master_and_rejects_known_wrong_outcomes() {
    let task = Task::User(Kind::Read);
    let mut run = run_paused(execute(task, &read_with_a_rejected_fragment()));
    run.check().unwrap();

    // the model's expectation, written out
    let only = |allowed: &[Allowed]| {
        assert_eq!(allowed.len(), 1);
        allowed[0].obs.clone()
    };
    let read = ReadType::SinglePoll;
    assert_eq!(
        only(&run.allowed[1]),
        StepObs {
            writes: vec![Write::Confirm(0)],
            reads: 1,
            main: vec![
                ReadEvent::BeginFragment(read),
                ReadEvent::AnalogInput,
                ReadEvent::EndFragment(read),
            ],
            ..StepObs::default()
        }
    );
    assert_eq!(
        only(&run.allowed[2]),
        StepObs {
            writes: vec![Write::Confirm(1)],
            reads: 1,
            main: vec![ReadEvent::BeginFragment(read), ReadEvent::EndFragment(read)],
            info: vec![AssocInfoEvent::TaskSuccess(
                TaskType::UserRead,
                FunctionCode::Read,
                Sequence::new(1)
            )],
            ..StepObs::default()
        }
    );
    assert_eq!(run.reply, ReplyObs::Done(Reply::Read(Ok(())), 2 * TICK));
    assert_eq!(
        run.probe,
        StepObs {
            writes: vec![Write::Request(FunctionCode::Read, 2)],
            info: vec![AssocInfoEvent::TaskStart(
                TaskType::UserRead,
                FunctionCode::Read,
                Sequence::new(2)
            )],
            ..StepObs::default()
        }
    );

    // confirming the last fragment with the first fragment's sequence number
    let observed = run.observed.clone();
    run.observed[2].writes = vec![Write::Confirm(0)];
    run.check().unwrap_err();
    run.observed = observed;

    // continuing the sequence numbers from the request rather than the last fragment
    let probe = run.probe.clone();
    run.probe.writes = vec![Write::Request(FunctionCode::Read, 1)];
    run.check().unwrap_err();
    run.probe = probe;

    // failing the read for the request error, though it returned data
    let rejected = TaskError::RejectedByIin2(Iin::new(Iin1::new(0), Iin2::new(0x02)));
    run.observed[2].info = vec![AssocInfoEvent::TaskFailure(TaskType::UserRead, rejected)];
    run.reply = ReplyObs::Done(Reply::Read(Err(rejected)), 2 * TICK);
    run.check().unwrap_err();
}

#[test]
fn a_startup_integrity_scan_times_out_one_response_timeout_after_its_last_fragment() {
    let first = Step {
        input: Input::Answer(Answer {
            source: Source::Outstation,
            seq: SeqChoice::Expected,
            fir: Fir::Proper,
            fin: false,
            con: true,
            restart: false,
            iin2: 0,
            objects: Objects::AnalogStatic,
        }),
        wait: FINAL_WAIT,
    };
    let run = run_paused(execute(Task::StartupIntegrity, &[first]));
    run.check().unwrap();

    let failed =
        AssocInfoEvent::TaskFailure(TaskType::StartupIntegrity, TaskError::ResponseTimeout);
    // the fragment at 7 ms and nothing else until a millisecond before its timeout at 1007 ms,
    // then the failure
    assert_eq!(run.observed.len(), 3);
    assert_eq!(run.observed[1].writes, vec![Write::Confirm(0)]);
    assert_eq!(run.observed[1].info, vec![]);
    assert_eq!(
        run.observed[2],
        StepObs {
            info: vec![failed],
            ..StepObs::default()
        }
    );
    assert_eq!(run.reply, ReplyObs::None);

    // timing out before 1007 ms, from the request at 0 ms
    let mut early = run;
    early.observed[1].info = vec![failed];
    early.observed[2].info = vec![];
    early.check().unwrap_err();
}

#[test]
fn the_model_rejects_a_missing_confirm_of_a_control_response() {
    let task = Task::User(Kind::DirectOperate);
    let response = Step {
        input: Input::Answer(Answer {
            source: Source::Outstation,
            seq: SeqChoice::Expected,
            fir: Fir::Proper,
            fin: true,
            con: true,
            restart: false,
            iin2: 0,
            objects: Objects::Correct,
        }),
        wait: TICK,
    };
    let mut run = run_paused(execute(task, &[response]));
    run.check().unwrap();
    assert_eq!(run.observed[1].writes, vec![Write::Confirm(0)]);
    assert_eq!(run.reply, ReplyObs::Done(Reply::Command(Ok(())), TICK));

    run.observed[1].writes.clear();
    run.check().unwrap_err();
}
