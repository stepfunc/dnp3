use crate::app::format::write::HeaderWriter;
use crate::master::types::{AutoRequest, ReadRequest};
use crate::util::cursor::WriteError;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(crate) struct PollState {
    value: std::rc::Rc<std::cell::Cell<Option<Instant>>>,
}

impl PollState {
    pub(crate) fn new(start: Option<Instant>) -> Self {
        Self {
            value: std::rc::Rc::new(std::cell::Cell::new(start)),
        }
    }

    pub(crate) fn set(&mut self, next: Option<Instant>) {
        self.value.set(next)
    }

    pub(crate) fn get(&self) -> Option<Instant> {
        self.value.get()
    }
}

#[derive(Clone)]
pub(crate) struct Poll {
    request: ReadRequest,
    period: Duration,
    next: PollState,
}

impl Poll {
    pub(crate) fn new(request: ReadRequest, period: Duration) -> Self {
        Self {
            request,
            period,
            next: PollState::new(Instant::now().checked_add(period)),
        }
    }

    pub(crate) fn write(&self, writer: &mut HeaderWriter) -> Result<(), WriteError> {
        self.request.format(writer)
    }

    pub(crate) fn success(&mut self) {
        self.next.set(Instant::now().checked_add(self.period))
    }

    pub(crate) fn to_request(&self) -> AutoRequest {
        AutoRequest::PeriodicPoll(self.clone())
    }
}
