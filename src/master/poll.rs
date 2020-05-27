use crate::app::format::write::HeaderWriter;
use crate::master::association::Next;
use crate::master::error::PollError;
use crate::master::handle::{Message, Promise};
use crate::master::request::ReadRequest;
use crate::util::cursor::WriteError;
use crate::util::Smallest;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Copy, Clone)]
pub(crate) struct Poll {
    pub(crate) id: u64,
    request: ReadRequest,
    period: Duration,
    next: Option<Instant>,
}

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
                return Next::Now(*poll);
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

pub(crate) struct PollTaskMsg {
    pub(crate) address: u16,
    pub(crate) task: PollTask,
}

impl PollTaskMsg {
    pub(crate) fn new(address: u16, task: PollTask) -> Self {
        Self { address, task }
    }
}

impl From<PollTaskMsg> for Message {
    fn from(msg: PollTaskMsg) -> Self {
        Self::PollTask(msg)
    }
}

pub(crate) enum PollTask {
    AddPoll(
        ReadRequest,
        Duration,
        tokio::sync::mpsc::Sender<Message>,
        Promise<Result<PollHandle, PollError>>,
    ),
    RemovePoll(u64),
    Demand(u64),
}

impl PollTask {
    pub(crate) fn on_error(self, err: PollError) {
        match self {
            PollTask::AddPoll(_, _, _, callback) => callback.complete(Err(err)),
            PollTask::RemovePoll(_) => {}
            PollTask::Demand(_) => {}
        }
    }
}

#[derive(Debug, Clone)]
pub struct PollHandle {
    channel: tokio::sync::mpsc::Sender<Message>,
    address: u16,
    id: u64,
}

impl PollHandle {
    pub(crate) fn new(channel: tokio::sync::mpsc::Sender<Message>, address: u16, id: u64) -> Self {
        Self {
            channel,
            address,
            id,
        }
    }

    pub async fn demand(&mut self) {
        self.channel
            .send(PollTaskMsg::new(self.address, PollTask::Demand(self.id)).into())
            .await
            .ok();
    }

    pub async fn remove(mut self) {
        self.channel
            .send(PollTaskMsg::new(self.address, PollTask::RemovePoll(self.id)).into())
            .await
            .ok();
    }
}
