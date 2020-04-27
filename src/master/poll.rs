use crate::app::format::write::HeaderWriter;
use crate::master::association::Next;
use crate::master::types::ReadRequest;
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

    pub(crate) fn add(&mut self, request: ReadRequest, period: Duration) {
        let id = self.id;
        self.id += 1;
        self.polls.insert(id, Poll::new(id, request, period));
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
