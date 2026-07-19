use std::collections::BTreeMap;
use std::time::Duration;

use crate::app::format::write::HeaderWriter;
use crate::app::Shutdown;
use crate::master::association::Next;
use crate::master::error::PollError;
use crate::master::handler::AssociationHandle;
use crate::master::request::ReadRequest;
use crate::util::Smallest;

use crate::master::promise::Promise;
use tokio::time::Instant;

/// Periodic poll representation
#[derive(Clone)]
pub(crate) struct Poll {
    /// Unique ID of the poll
    pub(crate) id: u64,
    /// Read request to perform
    request: ReadRequest,
    /// Period to wait between two polls
    period: Duration,
    /// Next instant to send the request (`None` if it was just created)
    next: Option<Instant>,
}

/// Map of all the polls of an association
pub(crate) struct PollMap {
    timer_state: TimerState,
    id: u64,
    polls: BTreeMap<u64, Poll>,
}

#[derive(Copy, Clone, PartialEq)]
enum TimerState {
    Deferred,
    Running,
}

impl PollMap {
    pub(crate) fn new() -> Self {
        Self {
            timer_state: TimerState::Running,
            id: 0,
            polls: BTreeMap::new(),
        }
    }

    pub(crate) fn new_deferred() -> Self {
        Self {
            timer_state: TimerState::Deferred,
            id: 0,
            polls: BTreeMap::new(),
        }
    }

    pub(crate) fn add(&mut self, request: ReadRequest, period: Duration) -> u64 {
        let id = self.id;
        self.id += 1;
        let poll = match self.timer_state {
            TimerState::Deferred => Poll::new_deferred(id, request, period),
            TimerState::Running => Poll::new(id, request, period),
        };
        self.polls.insert(id, poll);
        id
    }

    /// Start all timers that were deferred during configuration.
    ///
    /// Returns `true` only when this call transitions the map to running.
    pub(crate) fn start(&mut self, now: Instant) -> bool {
        if self.timer_state == TimerState::Running {
            return false;
        }

        self.timer_state = TimerState::Running;
        for poll in self.polls.values_mut() {
            poll.start(now);
        }
        true
    }

    pub(crate) fn remove(&mut self, id: u64) -> bool {
        self.polls.remove(&id).is_some()
    }

    pub(crate) fn demand(&mut self, id: u64) -> bool {
        if let Some(poll) = self.polls.get_mut(&id) {
            poll.demand();
            true
        } else {
            false
        }
    }

    pub(crate) fn complete(&mut self, id: u64) {
        if let Some(x) = self.polls.get_mut(&id) {
            x.reset_next()
        }
    }

    pub(crate) fn next(&self, now: Instant) -> Next<Poll> {
        let mut earliest = Smallest::<Instant>::new();

        for poll in self.polls.values() {
            if poll.is_ready(now) {
                return Next::Now(poll.clone());
            }

            if let Some(x) = poll.next() {
                earliest.observe(x)
            }
        }

        if let Some(x) = earliest.value() {
            return Next::NotBefore(x);
        }

        Next::None
    }
}

impl Poll {
    pub(crate) fn new(id: u64, request: ReadRequest, period: Duration) -> Self {
        Self {
            id,
            request,
            period,
            next: Instant::now().checked_add(period),
        }
    }

    fn new_deferred(id: u64, request: ReadRequest, period: Duration) -> Self {
        Self {
            id,
            request,
            period,
            next: None,
        }
    }

    fn start(&mut self, now: Instant) {
        self.next = now.checked_add(self.period);
    }

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), scursor::WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn demand(&mut self) {
        self.next = Some(Instant::now());
    }

    pub(crate) fn reset_next(&mut self) {
        self.next = Instant::now().checked_add(self.period)
    }

    pub(crate) fn is_ready(&self, now: Instant) -> bool {
        if let Some(next) = self.next {
            return next <= now;
        }
        false
    }

    pub(crate) fn next(&self) -> Option<Instant> {
        self.next
    }
}

pub(crate) enum PollMsg {
    AddPoll(
        AssociationHandle,
        ReadRequest,
        Duration,
        Promise<Result<PollHandle, PollError>>,
    ),
    RemovePoll(u64),
    Demand(u64),
}

impl PollMsg {
    pub(crate) fn on_error(self, err: PollError) {
        match self {
            PollMsg::AddPoll(_, _, _, callback) => callback.complete(Err(err)),
            PollMsg::RemovePoll(_) => {}
            PollMsg::Demand(_) => {}
        }
    }
}

/// Handle to a poll bound to a master association
#[derive(Clone)]
pub struct PollHandle {
    association: AssociationHandle,
    id: u64,
}

impl PollHandle {
    /// FFI only
    #[doc(hidden)]
    #[cfg(feature = "ffi")]
    pub fn get_id(&self) -> u64 {
        self.id
    }

    /// FFI only
    #[doc(hidden)]
    #[cfg(feature = "ffi")]
    pub fn create(association: AssociationHandle, id: u64) -> Self {
        Self::new(association, id)
    }

    pub(crate) fn new(association: AssociationHandle, id: u64) -> Self {
        Self { association, id }
    }

    /// Flag the poll for immediate execution prior to its period elapsing
    pub async fn demand(&mut self) -> Result<(), Shutdown> {
        self.association
            .send_poll_message(PollMsg::Demand(self.id))
            .await
    }

    /// Remove the poll from the association
    pub async fn remove(mut self) -> Result<(), Shutdown> {
        self.association
            .send_poll_message(PollMsg::RemovePoll(self.id))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::master::Classes;

    fn request() -> ReadRequest {
        ReadRequest::ClassScan(Classes::all())
    }

    #[test]
    fn deferred_polls_are_scheduled_from_start_time() {
        let period = Duration::from_secs(5);
        let start = Instant::now();
        let mut polls = PollMap::new_deferred();
        polls.add(request(), period);

        assert!(matches!(polls.next(start), Next::None));
        assert!(polls.start(start));

        match polls.next(start) {
            Next::NotBefore(deadline) => assert_eq!(deadline, start + period),
            _ => panic!("expected a scheduled poll"),
        }
    }

    #[test]
    fn starting_twice_does_not_reset_poll_deadlines() {
        let period = Duration::from_secs(5);
        let start = Instant::now();
        let mut polls = PollMap::new_deferred();
        polls.add(request(), period);

        assert!(polls.start(start));
        assert!(!polls.start(start + Duration::from_secs(1)));

        match polls.next(start) {
            Next::NotBefore(deadline) => assert_eq!(deadline, start + period),
            _ => panic!("expected a scheduled poll"),
        }
    }
}
