// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Property test of how a master channel picks its next task (`AssociationMap::next_task`),
//! against a model written from the scheduling guide (`guide/docs/api/master/scheduling.mdx`).
//!
//! The associations run without I/O on a paused clock. Each step asks the scheduler for the next
//! task and compares it with the model's prediction. The task is then answered, or times out,
//! through the calls the session makes, while generated events (unsolicited IIN bits, requests,
//! poll changes and time) arrive, and the model sees the same events.
//!
//! The model's rules:
//! - user requests of every association go before anything else, oldest first per association;
//! - associations take turns in the order added, and the one that starts a task goes to the back;
//! - within an association: clear restart, disable unsolicited, time sync, integrity scan, enable
//!   unsolicited, event scan, then the first due poll in the order added, then the keep-alive;
//! - a failed automatic task retries after a delay that starts at the minimum and doubles with
//!   each consecutive failure up to the maximum, and blocks everything after it until then;
//! - a failed time sync blocks nothing and retries after the integrity scan, so a failed task
//!   ahead of it holds up its retry;
//! - a poll runs one period after it's added or after its last run ends, at once when demanded,
//!   and again as soon as a run ends if it was demanded during the run;
//! - when nothing is ready, the channel sleeps until the earliest time something becomes ready.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "the model adds delays to instants on a paused clock and counts failures"
)]

use std::collections::VecDeque;

use proptest::prelude::*;
use scursor::WriteCursor;
use tokio::sync::oneshot;

use super::*;
use crate::app::format::write::start_response;
use crate::app::parse::options::ParseOptions;
use crate::app::parse::parser::ParsedFragment;
use crate::app::{ControlField, Iin1, Iin2, ResponseFunction};
use crate::link::error::LinkError;
use crate::master::request::ReadRequest;
use crate::master::task::ReadOutcome;
use crate::master::tasks::read::SingleReadTask;
use crate::master::tasks::NonReadTask;
use crate::master::test_fixtures::association;
use crate::util::test_support::run_paused;

/// What the channel starts, as the model names it
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Work {
    User,
    ClearRestart,
    DisableUnsolicited,
    TimeSync,
    Integrity,
    EnableUnsolicited,
    EventScan(EventClasses),
    /// a poll, by the order it was added to its association
    Poll(usize),
    KeepAlive,
    /// a task the model never schedules
    Unexpected,
}

/// What `next_task` returned, with associations named by the order they were added
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Observed {
    Run { assoc: usize, work: Work },
    NotBefore(Instant),
    Nothing,
}

/// What the model expects `next_task` to return
#[derive(Clone, Debug, PartialEq)]
enum Prediction {
    Run {
        assoc: usize,
        work: Work,
        /// the id of the user request
        user: Option<u32>,
    },
    /// Sleep until the earliest time some work becomes ready
    Sleep(Instant),
    Nothing,
}

impl Prediction {
    fn accepts(&self, now: Instant, observed: Observed) -> bool {
        match (self, observed) {
            (Self::Run { assoc, work, .. }, Observed::Run { assoc: a, work: w }) => {
                *assoc == a && *work == w
            }
            (Self::Sleep(wake), Observed::NotBefore(at)) => at > now && at == *wake,
            (Self::Nothing, Observed::Nothing) => true,
            _ => false,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Config {
    disable_unsolicited: bool,
    enable_unsolicited: bool,
    integrity: bool,
    time_sync: bool,
    /// whether the association handler can provide the time for a time sync
    has_clock: bool,
    integrity_on_overflow: bool,
    event_scan: EventClasses,
    keep_alive: Option<Duration>,
    retry_min: Duration,
    retry_max: Duration,
}

impl Config {
    /// The delay before the retry that follows `failures` consecutive failures
    fn retry_delay(&self, failures: u32) -> Duration {
        let factor = 1u32.checked_shl(failures - 1).unwrap_or(u32::MAX);
        self.retry_min
            .checked_mul(factor)
            .map_or(self.retry_max, |delay| delay.min(self.retry_max))
    }

    fn association_config(&self) -> AssociationConfig {
        fn classes(enabled: bool) -> EventClasses {
            if enabled {
                EventClasses::all()
            } else {
                EventClasses::none()
            }
        }

        AssociationConfig {
            response_timeout: Timeout::default(),
            disable_unsol_classes: classes(self.disable_unsolicited),
            enable_unsol_classes: classes(self.enable_unsolicited),
            startup_integrity_classes: if self.integrity {
                Classes::all()
            } else {
                Classes::none()
            },
            auto_time_sync: self
                .time_sync
                .then_some(TimeSyncProcedure::DirectWriteAbsTime),
            auto_tasks_retry_strategy: RetryStrategy::new(self.retry_min, self.retry_max),
            keep_alive_timeout: self.keep_alive,
            auto_integrity_scan_on_buffer_overflow: self.integrity_on_overflow,
            event_scan_on_events_available: self.event_scan,
            max_queued_user_requests: 1000,
        }
    }
}

/// The IIN bits a response carries
#[derive(Copy, Clone, Debug, Default)]
struct IinBits {
    restart: bool,
    need_time: bool,
    class1: bool,
    class2: bool,
    class3: bool,
    overflow: bool,
    /// one of the IIN2 request errors, by bit (0 to 2)
    reject: Option<u8>,
    config_corrupt: bool,
}

impl IinBits {
    fn iin(&self) -> Iin {
        fn bit(set: bool, position: u8) -> u8 {
            u8::from(set) << position
        }

        // IIN1: class 1 to 3 events are bits 1 to 3, NEED_TIME is bit 4, DEVICE_RESTART is bit 7.
        // IIN2: the request errors are bits 0 to 2, EVENT_BUFFER_OVERFLOW is bit 3,
        // CONFIG_CORRUPT is bit 5
        let iin1 = bit(self.class1, 1)
            | bit(self.class2, 2)
            | bit(self.class3, 3)
            | bit(self.need_time, 4)
            | bit(self.restart, 7);
        let iin2 = self.reject.map_or(0, |position| bit(true, position))
            | bit(self.overflow, 3)
            | bit(self.config_corrupt, 5);
        Iin::new(Iin1::new(iin1), Iin2::new(iin2))
    }

    fn events(&self) -> EventClasses {
        EventClasses::new(self.class1, self.class2, self.class3)
    }
}

#[derive(Copy, Clone, Debug)]
struct Answer {
    iin: IinBits,
    /// whether a read's response carried events
    events: bool,
}

#[derive(Copy, Clone, Debug)]
enum Outcome {
    Timeout,
    Answer(Answer),
}

/// The state of an automatic task
#[derive(Copy, Clone, Debug, PartialEq)]
enum Auto {
    Idle,
    Pending,
    /// failed `failures` times in a row, and retries at `at`
    Retry {
        failures: u32,
        at: Instant,
    },
}

impl Auto {
    fn demand(&mut self) {
        if *self == Self::Idle {
            *self = Self::Pending;
        }
    }

    fn fail(&mut self, now: Instant, config: &Config) {
        let failures = match *self {
            Self::Retry { failures, .. } => failures + 1,
            Self::Idle | Self::Pending => 1,
        };
        *self = Self::Retry {
            failures,
            at: now + config.retry_delay(failures),
        };
    }
}

#[derive(Copy, Clone, Debug)]
struct ModelPoll {
    period: Duration,
    next: Instant,
}

/// What an association does next
#[derive(Clone, Debug, PartialEq)]
enum AssocNext {
    Run(Work),
    /// nothing is ready until this time, if anything is waiting
    Sleep(Option<Instant>),
}

fn earliest(lhs: Option<Instant>, rhs: Option<Instant>) -> Option<Instant> {
    match (lhs, rhs) {
        (Some(lhs), Some(rhs)) => Some(lhs.min(rhs)),
        (lhs, None) => lhs,
        (None, rhs) => rhs,
    }
}

#[derive(Clone, Debug)]
struct ModelAssociation {
    config: Config,
    clear_restart: Auto,
    disable_unsolicited: Auto,
    time_sync: Auto,
    integrity: Auto,
    enable_unsolicited: Auto,
    event_scan: Auto,
    /// the event classes the last response reported
    events: EventClasses,
    /// the polls in the order added, `None` once removed
    polls: Vec<Option<ModelPoll>>,
    keep_alive_at: Option<Instant>,
    /// ids of the queued user requests, oldest first
    requests: VecDeque<u32>,
}

impl ModelAssociation {
    fn new(config: Config, now: Instant) -> Self {
        let mut association = Self {
            config,
            clear_restart: Auto::Idle,
            disable_unsolicited: Auto::Idle,
            time_sync: Auto::Idle,
            integrity: Auto::Idle,
            enable_unsolicited: Auto::Idle,
            event_scan: Auto::Idle,
            events: EventClasses::none(),
            polls: Vec::new(),
            keep_alive_at: config.keep_alive.map(|timeout| now + timeout),
            requests: VecDeque::new(),
        };
        association.start_communicating();
        association
    }

    /// The association (re)starts communicating: the startup sequence runs from the beginning
    fn start_communicating(&mut self) {
        self.clear_restart = Auto::Idle;
        self.disable_unsolicited = Auto::Pending;
        self.time_sync = Auto::Idle;
        self.integrity = Auto::Pending;
        self.enable_unsolicited = Auto::Pending;
        self.event_scan = Auto::Idle;
        self.requests.clear();
    }

    fn events_to_scan(&self) -> EventClasses {
        self.events & self.config.event_scan
    }

    /// Something was received from the outstation
    fn received(&mut self, now: Instant) {
        self.keep_alive_at = self.config.keep_alive.map(|timeout| now + timeout);
    }

    fn observe_iin(&mut self, iin: IinBits) {
        if iin.restart {
            self.clear_restart.demand();
            self.integrity.demand();
            self.enable_unsolicited.demand();
        }
        if iin.need_time {
            self.time_sync.demand();
        }
        if iin.overflow && self.config.integrity_on_overflow {
            self.integrity.demand();
        }
        self.events = iin.events();
        if self.events_to_scan().any() {
            self.event_scan.demand();
        }
    }

    /// The next work, after failing any time sync that can't start for lack of a clock
    fn next(&mut self, now: Instant) -> AssocNext {
        loop {
            match self.select(now) {
                AssocNext::Run(Work::TimeSync) if !self.config.has_clock => {
                    let config = self.config;
                    self.time_sync.fail(now, &config);
                }
                next => return next,
            }
        }
    }

    fn select(&self, now: Instant) -> AssocNext {
        let config = &self.config;
        let time_sync_failed = matches!(self.time_sync, Auto::Retry { .. });
        let to_scan = self.events_to_scan();

        // (work, state, whether it applies)
        let order = [
            (Work::ClearRestart, self.clear_restart, true),
            (
                Work::DisableUnsolicited,
                self.disable_unsolicited,
                config.disable_unsolicited,
            ),
            (
                Work::TimeSync,
                self.time_sync,
                config.time_sync && !time_sync_failed,
            ),
            (Work::Integrity, self.integrity, config.integrity),
            (
                Work::TimeSync,
                self.time_sync,
                config.time_sync && time_sync_failed,
            ),
            (
                Work::EnableUnsolicited,
                self.enable_unsolicited,
                config.enable_unsolicited,
            ),
            (Work::EventScan(to_scan), self.event_scan, to_scan.any()),
        ];

        // a failed time sync that isn't due blocks nothing, but once nothing ahead of it holds it
        // up, the channel must wake for its retry
        let mut time_sync_retry = None;
        for (work, state, applies) in order {
            if !applies {
                continue;
            }
            match state {
                Auto::Idle => {}
                Auto::Pending => return AssocNext::Run(work),
                Auto::Retry { at, .. } if at <= now => return AssocNext::Run(work),
                Auto::Retry { at, .. } if work == Work::TimeSync => time_sync_retry = Some(at),
                // it holds up everything after it
                Auto::Retry { at, .. } => {
                    return AssocNext::Sleep(earliest(Some(at), time_sync_retry))
                }
            }
        }

        for (index, poll) in self.polls.iter().enumerate() {
            if let Some(poll) = poll {
                if poll.next <= now {
                    return AssocNext::Run(Work::Poll(index));
                }
            }
        }

        if let Some(at) = self.keep_alive_at {
            if at <= now {
                return AssocNext::Run(Work::KeepAlive);
            }
        }

        let mut wake = earliest(time_sync_retry, self.keep_alive_at);
        for poll in self.polls.iter().flatten() {
            wake = earliest(wake, Some(poll.next));
        }
        AssocNext::Sleep(wake)
    }

    /// The task `run` completed at `now` with this outcome
    fn complete(&mut self, run: &Running, outcome: &Outcome, now: Instant) {
        let config = self.config;
        let answer = match outcome {
            Outcome::Timeout => {
                self.failed(run, now);
                return;
            }
            Outcome::Answer(answer) => answer,
        };

        self.received(now);
        if run.work != Work::KeepAlive {
            self.observe_iin(answer.iin);
        }

        let rejected = answer.iin.reject.is_some();
        // an outstation restart during the integrity scan or ENABLE_UNSOLICITED voids the run
        let restarted = run.restart_seen || answer.iin.restart;
        match run.work {
            Work::User | Work::KeepAlive | Work::Unexpected => {}
            Work::Poll(index) => self.poll_ran(index, run.poll_demanded, now),
            Work::ClearRestart => {
                if answer.iin.restart {
                    self.clear_restart.fail(now, &config);
                } else {
                    self.clear_restart = Auto::Idle;
                }
            }
            Work::DisableUnsolicited => self.disable_unsolicited = Auto::Idle,
            Work::TimeSync => {
                if answer.iin.need_time || rejected {
                    self.time_sync.fail(now, &config);
                } else {
                    self.time_sync = Auto::Idle;
                }
            }
            Work::Integrity => self.integrity = if restarted { Auto::Pending } else { Auto::Idle },
            Work::EnableUnsolicited => {
                self.enable_unsolicited = if restarted { Auto::Pending } else { Auto::Idle }
            }
            Work::EventScan(_) => {
                // scan again at once only if this scan returned events, wasn't rejected, and its
                // response still reports events to scan
                self.event_scan = if answer.events && !rejected && self.events_to_scan().any() {
                    Auto::Pending
                } else {
                    Auto::Idle
                }
            }
        }
    }

    fn failed(&mut self, run: &Running, now: Instant) {
        let config = self.config;
        match run.work {
            Work::User | Work::KeepAlive | Work::Unexpected => {}
            Work::Poll(index) => self.poll_ran(index, run.poll_demanded, now),
            Work::ClearRestart => self.clear_restart.fail(now, &config),
            Work::DisableUnsolicited => self.disable_unsolicited.fail(now, &config),
            Work::TimeSync => self.time_sync.fail(now, &config),
            Work::Integrity => self.integrity.fail(now, &config),
            Work::EnableUnsolicited => self.enable_unsolicited.fail(now, &config),
            Work::EventScan(_) => self.event_scan.fail(now, &config),
        }
    }

    fn poll_ran(&mut self, index: usize, demanded: bool, now: Instant) {
        if let Some(Some(poll)) = self.polls.get_mut(index) {
            // a poll demanded while it ran stays due from the demand
            if !demanded {
                poll.next = now + poll.period;
            }
        }
    }
}

/// The task the channel is running
#[derive(Copy, Clone, Debug)]
struct Running {
    assoc: usize,
    work: Work,
    user: Option<u32>,
    poll_demanded: bool,
    /// whether the outstation reported a restart while the task ran
    restart_seen: bool,
}

/// Which associations had work ready before a pick, in line order
#[derive(Copy, Clone, Debug)]
struct Ready {
    assoc: usize,
    user: bool,
    auto: bool,
}

#[derive(Default)]
struct Model {
    /// associations by the order added, `None` once removed
    associations: Vec<Option<ModelAssociation>>,
    /// associations in the order they take turns
    line: VecDeque<usize>,
    next_request: u32,
}

impl Model {
    fn add(&mut self, config: Config, now: Instant) -> usize {
        let index = self.associations.len();
        self.associations
            .push(Some(ModelAssociation::new(config, now)));
        self.line.push_back(index);
        index
    }

    fn remove(&mut self, index: usize) {
        self.associations[index] = None;
        self.line.retain(|x| *x != index);
    }

    fn association(&mut self, index: usize) -> Option<&mut ModelAssociation> {
        self.associations.get_mut(index).and_then(Option::as_mut)
    }

    /// The `n`th association still present, wrapping around
    fn live(&self, n: usize) -> Option<usize> {
        let live: Vec<usize> = (0..self.associations.len())
            .filter(|x| self.associations[*x].is_some())
            .collect();
        if live.is_empty() {
            None
        } else {
            Some(live[n % live.len()])
        }
    }

    fn queue_request(&mut self, index: usize) -> u32 {
        let id = self.next_request;
        self.next_request += 1;
        if let Some(association) = self.association(index) {
            association.requests.push_back(id);
        }
        id
    }

    fn reconnect(&mut self) {
        for association in self.associations.iter_mut().flatten() {
            association.start_communicating();
        }
    }

    fn take_turn(&mut self, position: usize) {
        if let Some(x) = self.line.remove(position) {
            self.line.push_back(x);
        }
    }

    fn ready(&self, now: Instant) -> Vec<Ready> {
        self.line
            .iter()
            .filter_map(|index| {
                let association = self.associations[*index].as_ref()?;
                Some(Ready {
                    assoc: *index,
                    user: !association.requests.is_empty(),
                    auto: matches!(association.clone().next(now), AssocNext::Run(_)),
                })
            })
            .collect()
    }

    fn pick(&mut self, now: Instant) -> Prediction {
        for position in 0..self.line.len() {
            let index = self.line[position];
            let user = self.association(index).and_then(|x| x.requests.pop_front());
            if let Some(id) = user {
                self.take_turn(position);
                return Prediction::Run {
                    assoc: index,
                    work: Work::User,
                    user: Some(id),
                };
            }
        }

        let mut wake = None;
        for position in 0..self.line.len() {
            let index = self.line[position];
            let Some(association) = self.association(index) else {
                continue;
            };
            match association.next(now) {
                AssocNext::Run(work) => {
                    self.take_turn(position);
                    return Prediction::Run {
                        assoc: index,
                        work,
                        user: None,
                    };
                }
                AssocNext::Sleep(at) => wake = earliest(wake, at),
            }
        }

        match wake {
            Some(wake) => Prediction::Sleep(wake),
            None => Prediction::Nothing,
        }
    }
}

/// How many picks each association has waited with ready work, by class of work
#[derive(Copy, Clone, Debug)]
struct Wait {
    picks: usize,
    /// the associations ahead of it in line when it started waiting
    ahead: usize,
}

#[derive(Default)]
struct Fairness {
    /// by association, for user requests and for other work
    waits: Vec<[Option<Wait>; 2]>,
}

impl Fairness {
    /// An association with ready work is served before every association behind it in line
    /// with ready work of the same class, so it waits at most one pick per association ahead
    fn check(&mut self, ready: &[Ready], observed: Observed) -> Result<(), TestCaseError> {
        let (picked, served) = match observed {
            Observed::Run { assoc, work } => (Some(assoc), Some(usize::from(work != Work::User))),
            Observed::NotBefore(_) | Observed::Nothing => (None, None),
        };

        for (ahead, ready) in ready.iter().enumerate() {
            if self.waits.len() <= ready.assoc {
                self.waits.resize(ready.assoc + 1, [None, None]);
            }
            for (class, is_ready) in [ready.user, ready.auto].into_iter().enumerate() {
                let wait = &mut self.waits[ready.assoc][class];
                if !is_ready || picked == Some(ready.assoc) {
                    *wait = None;
                    continue;
                }
                if served == Some(class) {
                    let wait = wait.get_or_insert(Wait { picks: 0, ahead });
                    wait.picks += 1;
                    prop_assert!(
                        wait.picks <= wait.ahead,
                        "association {} waited {:?}",
                        ready.assoc,
                        wait
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Copy, Clone, Debug)]
enum Event {
    Advance(Duration),
    /// queue a user request on the nth association
    Request(usize),
    /// an unsolicited response from the nth association
    Unsolicited(usize, IinBits),
    AddPoll(usize, Duration),
    /// demand the poll of the nth association, by the order added
    DemandPoll(usize, usize),
    RemovePoll(usize, usize),
}

#[derive(Clone, Debug)]
enum Op {
    /// ask for the next task; if it runs, these events happen before it completes
    Step(Vec<Event>, Outcome),
    Event(Event),
    /// advance to the deadline the scheduler last asked to sleep until
    Sleep,
    Reconnect,
    AddAssociation(Config),
    RemoveAssociation(usize),
}

/// The associations under test
struct System {
    map: AssociationMap,
    /// by the order added
    addresses: Vec<EndpointAddress>,
    /// poll ids by association and the order added
    polls: Vec<Vec<u64>>,
    /// the results of the user requests, by id
    requests: Vec<oneshot::Receiver<Result<(), TaskError>>>,
}

struct Clock(bool);

impl AssociationHandler for Clock {
    fn get_current_time(&self) -> Option<Timestamp> {
        self.0.then_some(Timestamp::new(0))
    }
}

impl System {
    fn new() -> Self {
        Self {
            map: AssociationMap::new(),
            addresses: Vec::new(),
            polls: Vec::new(),
            requests: Vec::new(),
        }
    }

    fn add(&mut self, config: &Config, address: u16) {
        let link = EndpointAddress::try_new(address).unwrap();
        let association = association(
            link,
            config.association_config(),
            Box::new(Clock(config.has_clock)),
        );
        self.map.register(association).unwrap();
        self.addresses.push(link);
        self.polls.push(Vec::new());
    }

    fn association(&mut self, index: usize) -> &mut Association {
        self.map.get_mut(self.addresses[index]).ok().unwrap()
    }

    fn queue_request(&mut self, index: usize) {
        let (promise, reply) = Promise::one_shot();
        let task = SingleReadTask::new(ReadRequest::ClassScan(Classes::all()), promise);
        self.association(index)
            .process_message(AssociationMsgType::QueueTask(Task::from(task).into()), true);
        self.requests.push(reply);
    }

    fn observe(&self, next: &Next<AssociationTask>) -> Observed {
        let task = match next {
            Next::Now(task) => task,
            Next::NotBefore(at) => return Observed::NotBefore(*at),
            Next::None => return Observed::Nothing,
        };

        let assoc = self
            .addresses
            .iter()
            .position(|x| *x == task.dest.link)
            .unwrap();
        let work = match &task.details {
            Task::LinkStatus(_) => Work::KeepAlive,
            Task::App(AppTask::Read(read)) => match read {
                ReadTask::SingleRead(_) => Work::User,
                ReadTask::StartupIntegrity(_) => Work::Integrity,
                ReadTask::EventScan(classes) => Work::EventScan(*classes),
                ReadTask::PeriodicPoll(poll) => self.polls[assoc]
                    .iter()
                    .position(|id| *id == poll.id)
                    .map_or(Work::Unexpected, Work::Poll),
            },
            Task::App(AppTask::NonRead(NonReadTask::Auto(task))) => match task {
                AutoTask::ClearRestartBit => Work::ClearRestart,
                AutoTask::DisableUnsolicited(_) => Work::DisableUnsolicited,
                AutoTask::EnableUnsolicited(_) => Work::EnableUnsolicited,
            },
            Task::App(AppTask::NonRead(NonReadTask::TimeSync(_))) => Work::TimeSync,
            Task::App(_) => Work::Unexpected,
        };
        Observed::Run { assoc, work }
    }

    /// Complete the task the way the session does, for this outcome
    async fn complete(
        &mut self,
        task: AssociationTask,
        outcome: &Outcome,
    ) -> Result<(), TestCaseError> {
        let Ok(association) = self.map.get_mut(task.dest.link) else {
            task.details
                .on_task_error(None, TaskError::NoSuchAssociation(task.dest.link));
            return Ok(());
        };
        let answer = match outcome {
            Outcome::Timeout => {
                task.details
                    .on_task_error(Some(association), TaskError::ResponseTimeout);
                return Ok(());
            }
            Outcome::Answer(answer) => answer,
        };

        association.on_link_activity();
        let iin = answer.iin.iin();
        match task.details {
            Task::LinkStatus(promise) => promise.complete(Ok(())),
            Task::App(AppTask::Read(read)) => {
                association.process_iin(iin);
                // a response with events delivers their values
                let mut outcome = ReadOutcome::default();
                outcome.add(
                    iin,
                    Delivered {
                        values: answer.events,
                        events: answer.events,
                    },
                );
                match outcome.result(&read) {
                    Ok(()) => read.complete(association, outcome.events_without_request_errors()),
                    Err(err) => read.on_task_error(Some(association), err),
                }
            }
            Task::App(AppTask::NonRead(task)) => {
                association.process_iin(iin);
                if iin.has_bad_request_error() {
                    task.on_task_error(Some(association), TaskError::RejectedByIin2(iin));
                } else {
                    let mut buffer = [0; 4];
                    let mut cursor = WriteCursor::new(&mut buffer);
                    start_response(
                        ControlField::response(Sequence::default(), true, true, false),
                        ResponseFunction::Response,
                        iin,
                        &mut cursor,
                    )
                    .unwrap();
                    let response = ParsedFragment::parse(ParseOptions::default(), &buffer)
                        .unwrap()
                        .to_response()
                        .unwrap();
                    let next = task.handle_response(association, response).await;
                    // every automatic task and the time sync procedure used here take one request
                    prop_assert!(!matches!(next, Ok(Some(_))));
                }
            }
            Task::App(task @ AppTask::NoResponse(_)) => {
                task.on_task_error(Some(association), TaskError::ResponseTimeout)
            }
        }
        Ok(())
    }
}

/// Apply an event to the model and the associations
async fn apply(
    model: &mut Model,
    system: &mut System,
    mut running: Option<&mut Running>,
    event: Event,
) {
    let now = Instant::now();
    match event {
        Event::Advance(delay) => tokio::time::advance(delay).await,
        Event::Request(n) => {
            if let Some(index) = model.live(n) {
                model.queue_request(index);
                system.queue_request(index);
            }
        }
        Event::Unsolicited(n, iin) => {
            if let Some(index) = model.live(n) {
                if let Some(association) = model.association(index) {
                    association.received(now);
                    association.observe_iin(iin);
                }
                if let Some(running) = running.as_deref_mut() {
                    if running.assoc == index && iin.restart {
                        running.restart_seen = true;
                    }
                }
                let association = system.association(index);
                association.on_link_activity();
                association.process_iin(iin.iin());
            }
        }
        Event::AddPoll(n, period) => {
            if let Some(index) = model.live(n) {
                if let Some(association) = model.association(index) {
                    association.polls.push(Some(ModelPoll {
                        period,
                        next: now + period,
                    }));
                }
                let id = system
                    .association(index)
                    .polls
                    .add(ReadRequest::ClassScan(Classes::all()), period);
                system.polls[index].push(id);
            }
        }
        Event::DemandPoll(n, poll) => {
            let Some(index) = model.live(n) else {
                return;
            };
            if system.polls[index].is_empty() {
                return;
            }
            let poll = poll % system.polls[index].len();
            if let Some(Some(model_poll)) =
                model.association(index).and_then(|x| x.polls.get_mut(poll))
            {
                model_poll.next = now;
                if let Some(running) = running {
                    if running.assoc == index && running.work == Work::Poll(poll) {
                        running.poll_demanded = true;
                    }
                }
            }
            let id = system.polls[index][poll];
            system
                .association(index)
                .process_message(AssociationMsgType::Poll(PollMsg::Demand(id)), true);
        }
        Event::RemovePoll(n, poll) => {
            let Some(index) = model.live(n) else {
                return;
            };
            if system.polls[index].is_empty() {
                return;
            }
            let poll = poll % system.polls[index].len();
            if let Some(association) = model.association(index) {
                association.polls[poll] = None;
            }
            let id = system.polls[index][poll];
            system
                .association(index)
                .process_message(AssociationMsgType::Poll(PollMsg::RemovePoll(id)), true);
        }
    }
}

#[derive(Clone, Debug)]
struct Case {
    associations: Vec<Config>,
    /// link addresses, in the order associations are added
    addresses: Vec<u16>,
    ops: Vec<Op>,
}

async fn run(case: Case) -> Result<(), TestCaseError> {
    let mut model = Model::default();
    let mut system = System::new();
    let mut fairness = Fairness::default();
    let mut wake = None;

    for config in &case.associations {
        system.add(config, case.addresses[model.associations.len()]);
        model.add(*config, Instant::now());
    }

    for op in &case.ops {
        match op {
            Op::Step(events, outcome) => {
                let now = Instant::now();
                let ready = model.ready(now);
                let prediction = model.pick(now);
                let next = system.map.next_task();
                let observed = system.observe(&next);
                prop_assert!(
                    prediction.accepts(now, observed),
                    "expected {:?}, observed {:?}",
                    prediction,
                    observed
                );
                fairness.check(&ready, observed)?;

                match (next, prediction) {
                    (Next::Now(task), Prediction::Run { assoc, work, user }) => {
                        let mut running = Running {
                            assoc,
                            work,
                            user,
                            poll_demanded: false,
                            restart_seen: false,
                        };
                        for event in events {
                            apply(&mut model, &mut system, Some(&mut running), *event).await;
                        }
                        let now = Instant::now();
                        if let Some(association) = model.association(running.assoc) {
                            association.complete(&running, outcome, now);
                        }
                        system.complete(task, outcome).await?;
                        if let Some(id) = running.user {
                            // the request the model expected is the one that ran
                            let reply = system.requests[usize::try_from(id).unwrap()].try_recv();
                            prop_assert!(reply.is_ok(), "user request {} didn't run", id);
                        }
                    }
                    (next, _) => {
                        if let Next::NotBefore(at) = next {
                            wake = Some(at);
                        }
                        for event in events {
                            apply(&mut model, &mut system, None, *event).await;
                        }
                    }
                }
            }
            Op::Event(event) => apply(&mut model, &mut system, None, *event).await,
            Op::Sleep => {
                if let Some(at) = wake.take() {
                    let now = Instant::now();
                    if at > now {
                        tokio::time::advance(at - now).await;
                    }
                }
            }
            Op::Reconnect => {
                model.reconnect();
                system.map.reset(RunError::Link(LinkError::Stdio(
                    std::io::ErrorKind::ConnectionReset,
                )));
            }
            Op::AddAssociation(config) => {
                if let Some(address) = case.addresses.get(model.associations.len()) {
                    system.add(config, *address);
                    model.add(*config, Instant::now());
                }
            }
            Op::RemoveAssociation(n) => {
                if let Some(index) = model.live(*n) {
                    model.remove(index);
                    system.map.remove(system.addresses[index]);
                }
            }
        }
    }

    Ok(())
}

const MAX_ASSOCIATIONS: u16 = 6;

fn millis(choices: &'static [u64]) -> impl Strategy<Value = Duration> {
    prop::sample::select(choices).prop_map(Duration::from_millis)
}

fn config() -> impl Strategy<Value = Config> {
    (
        (
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            prop::bool::weighted(0.75),
            any::<bool>(),
        ),
        (any::<bool>(), any::<bool>(), any::<bool>()),
        prop_oneof![
            2 => Just(None),
            1 => Just(Some(Duration::ZERO)),
            2 => millis(&[3000, 5000]).prop_map(Some),
        ],
        millis(&[1, 1000, 2000]),
        millis(&[1000, 4000, 10000]),
    )
        .prop_map(
            |(
                (
                    disable_unsolicited,
                    enable_unsolicited,
                    integrity,
                    time_sync,
                    has_clock,
                    integrity_on_overflow,
                ),
                (class1, class2, class3),
                keep_alive,
                retry_min,
                retry_max,
            )| Config {
                disable_unsolicited,
                enable_unsolicited,
                integrity,
                time_sync,
                has_clock,
                integrity_on_overflow,
                event_scan: EventClasses::new(class1, class2, class3),
                keep_alive,
                retry_min: retry_min.min(retry_max),
                retry_max: retry_min.max(retry_max),
            },
        )
}

fn iin() -> impl Strategy<Value = IinBits> {
    let bit = || prop::bool::weighted(0.2);
    (
        (bit(), bit(), bit(), bit(), bit(), bit()),
        prop_oneof![6 => Just(None), 1 => (0u8..3).prop_map(Some)],
        prop::bool::weighted(0.1),
    )
        .prop_map(
            |((restart, need_time, class1, class2, class3, overflow), reject, config_corrupt)| {
                IinBits {
                    restart,
                    need_time,
                    class1,
                    class2,
                    class3,
                    overflow,
                    reject,
                    config_corrupt,
                }
            },
        )
}

fn event() -> impl Strategy<Value = Event> {
    let assoc = 0..usize::from(MAX_ASSOCIATIONS);
    prop_oneof![
        3 => millis(&[0, 500, 1000, 2000, 3000, 5000, 10000]).prop_map(Event::Advance),
        2 => assoc.clone().prop_map(Event::Request),
        3 => (assoc.clone(), iin()).prop_map(|(n, iin)| Event::Unsolicited(n, iin)),
        2 => (assoc.clone(), millis(&[0, 1000, 2000, 5000]))
            .prop_map(|(n, period)| Event::AddPoll(n, period)),
        2 => (assoc.clone(), 0..4usize).prop_map(|(n, poll)| Event::DemandPoll(n, poll)),
        1 => (assoc, 0..4usize).prop_map(|(n, poll)| Event::RemovePoll(n, poll)),
    ]
}

fn outcome() -> impl Strategy<Value = Outcome> {
    prop_oneof![
        1 => Just(Outcome::Timeout),
        3 => (iin(), any::<bool>()).prop_map(|(iin, events)| Outcome::Answer(Answer { iin, events })),
    ]
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        10 => (prop::collection::vec(event(), 0..3), outcome())
            .prop_map(|(events, outcome)| Op::Step(events, outcome)),
        4 => event().prop_map(Op::Event),
        3 => Just(Op::Sleep),
        1 => Just(Op::Reconnect),
        1 => config().prop_map(Op::AddAssociation),
        1 => (0..usize::from(MAX_ASSOCIATIONS)).prop_map(Op::RemoveAssociation),
    ]
}

fn case() -> impl Strategy<Value = Case> {
    (
        prop::collection::vec(config(), 1..=3),
        Just((1..=MAX_ASSOCIATIONS).collect::<Vec<u16>>()).prop_shuffle(),
        prop::collection::vec(op(), 1..=40),
    )
        .prop_map(|(associations, addresses, ops)| Case {
            associations,
            addresses,
            ops,
        })
}

proptest! {
    #![proptest_config(crate::util::test_support::proptest_config(256))]

    #[test]
    fn next_task_follows_the_scheduling_model(case in case()) {
        run_paused(run(case))?;
    }
}

#[test]
fn sleeps_past_a_failed_time_sync_held_up_by_a_failed_integrity_scan() {
    let success = Outcome::Answer(Answer {
        iin: IinBits::default(),
        events: false,
    });
    let need_time = IinBits {
        need_time: true,
        ..IinBits::default()
    };
    let case = Case {
        associations: vec![Config {
            integrity: true,
            time_sync: true,
            ..quiet_config()
        }],
        addresses: vec![1],
        ops: vec![
            Op::Event(Event::Unsolicited(0, need_time)),
            // the time sync fails, then the integrity scan fails later, so it retries later
            Op::Step(Vec::new(), Outcome::Timeout),
            Op::Step(
                vec![Event::Advance(Duration::from_millis(500))],
                Outcome::Timeout,
            ),
            // the channel sleeps until the integrity scan retries, not until the time sync does
            Op::Step(Vec::new(), Outcome::Timeout),
            Op::Sleep,
            Op::Step(Vec::new(), success),
            Op::Step(Vec::new(), success),
        ],
    };
    run_paused(run(case)).unwrap();
}

fn quiet_config() -> Config {
    Config {
        disable_unsolicited: false,
        enable_unsolicited: false,
        integrity: false,
        time_sync: false,
        has_clock: true,
        integrity_on_overflow: false,
        event_scan: EventClasses::none(),
        keep_alive: None,
        retry_min: Duration::from_secs(1),
        retry_max: Duration::from_secs(4),
    }
}

#[test]
fn model_rejects_a_failed_time_sync_ahead_of_the_integrity_scan() {
    let now = Instant::now();
    let mut model = Model::default();
    let index = model.add(
        Config {
            integrity: true,
            time_sync: true,
            ..quiet_config()
        },
        now,
    );
    let association = model.association(index).unwrap();
    association.time_sync = Auto::Retry {
        failures: 1,
        at: now,
    };

    let prediction = model.pick(now);
    assert_eq!(
        prediction,
        Prediction::Run {
            assoc: index,
            work: Work::Integrity,
            user: None
        }
    );
    assert!(!prediction.accepts(
        now,
        Observed::Run {
            assoc: index,
            work: Work::TimeSync
        }
    ));
}

#[test]
fn model_rejects_a_poll_held_up_by_a_failed_integrity_scan_but_not_by_a_failed_time_sync() {
    let now = Instant::now();
    let retry = now + Duration::from_secs(1);
    let mut model = Model::default();
    let index = model.add(
        Config {
            integrity: true,
            time_sync: true,
            ..quiet_config()
        },
        now,
    );
    let association = model.association(index).unwrap();
    association.polls.push(Some(ModelPoll {
        period: Duration::from_secs(5),
        next: now,
    }));
    let poll = Observed::Run {
        assoc: index,
        work: Work::Poll(0),
    };

    association.integrity = Auto::Retry {
        failures: 1,
        at: retry,
    };
    let prediction = model.pick(now);
    assert_eq!(prediction, Prediction::Sleep(retry));
    assert!(!prediction.accepts(now, poll));

    let association = model.association(index).unwrap();
    association.integrity = Auto::Idle;
    association.time_sync = Auto::Retry {
        failures: 1,
        at: retry,
    };
    let prediction = model.pick(now);
    assert!(prediction.accepts(now, poll));
    assert!(!prediction.accepts(now, Observed::NotBefore(retry)));
}

#[test]
fn model_rejects_waking_for_a_failed_time_sync_held_up_by_a_failed_integrity_scan() {
    let now = Instant::now();
    let time_sync_retry = now + Duration::from_secs(1);
    let integrity_retry = now + Duration::from_secs(2);
    let mut model = Model::default();
    let index = model.add(
        Config {
            integrity: true,
            time_sync: true,
            ..quiet_config()
        },
        now,
    );
    let association = model.association(index).unwrap();
    association.time_sync = Auto::Retry {
        failures: 1,
        at: time_sync_retry,
    };
    association.integrity = Auto::Retry {
        failures: 1,
        at: integrity_retry,
    };

    let prediction = model.pick(now);
    assert_eq!(prediction, Prediction::Sleep(integrity_retry));
    assert!(!prediction.accepts(now, Observed::NotBefore(time_sync_retry)));
}

#[test]
fn retry_delays_double_from_the_minimum_up_to_the_maximum() {
    let config = quiet_config();
    let delays: Vec<u64> = (1..=5)
        .map(|failures| config.retry_delay(failures).as_secs())
        .collect();
    assert_eq!(delays, [1, 2, 4, 4, 4]);
}
