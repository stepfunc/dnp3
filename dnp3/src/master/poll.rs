use crate::app::format::write::HeaderWriter;
use crate::master::association::Next;
use crate::master::error::PollError;
use crate::master::handle::{AssociationHandle, Promise};
use crate::master::request::ReadRequest;
use crate::tokio::time::Instant;
use crate::util::cursor::WriteError;
use crate::util::Smallest;
use std::collections::BTreeMap;
use std::time::Duration;

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
    id: u64,
    polls: BTreeMap<u64, Poll>,
}

impl PollMap {
    pub(crate) fn new() -> Self {
        Self {
            id: 0,
            polls: BTreeMap::new(),
        }
    }

    pub(crate) fn add(&mut self, request: ReadRequest, period: Duration) -> u64 {
        let id = self.id;
        self.id += 1;
        self.polls.insert(id, Poll::new(id, request, period));
        id
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

    pub(crate) fn format(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
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

#[derive(Clone)]
pub struct PollHandle {
    association: AssociationHandle,
    id: u64,
}

impl PollHandle {
    pub(crate) fn new(association: AssociationHandle, id: u64) -> Self {
        Self { association, id }
    }

    pub async fn demand(&mut self) {
        self.association
            .send_poll_message(PollMsg::Demand(self.id))
            .await
            .ok();
    }

    pub async fn remove(mut self) {
        self.association
            .send_poll_message(PollMsg::RemovePoll(self.id))
            .await
            .ok();
    }
}
