use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, RequestCompletionHandler};
use crate::master::request::{MasterRequest, RequestDetails};
use crate::master::requests::basic::BasicRequestImpl;
use crate::master::requests::command::CommandRequestImpl;
use crate::master::requests::read::ReadRequestImpl;
use crate::master::types::{
    BasicRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};

struct Shared {
    destination: u16,
    seq: Sequence,
}

impl Shared {
    pub(crate) fn new(destination: u16) -> Self {
        Self {
            destination,
            seq: Sequence::default(),
        }
    }
}

#[derive(Clone)]
pub struct Session {
    shared: std::rc::Rc<std::cell::RefCell<Shared>>,
}

impl Session {
    pub fn new(destination: u16) -> Self {
        Self {
            shared: std::rc::Rc::new(std::cell::RefCell::new(Shared::new(destination))),
        }
    }

    pub fn destination(&self) -> u16 {
        self.shared.borrow().destination
    }
}

impl Session {
    pub fn read(&self, request: ReadRequest, handler: Box<dyn ReadTaskHandler>) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            RequestDetails::Read(ReadRequestImpl { request, handler }),
        )
    }

    pub fn seq(&self) -> u8 {
        self.shared.borrow().seq.value()
    }

    pub fn increment_seq(&mut self) -> Sequence {
        self.shared.borrow_mut().seq.increment()
    }

    pub fn previous_seq(&self) -> u8 {
        self.shared.borrow().seq.previous()
    }

    pub fn disable_unsolicited(
        &self,
        classes: EventClasses,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            RequestDetails::EmptyResponse(BasicRequestImpl::new(
                BasicRequest::DisableUnsolicited(classes),
                handler,
            )),
        )
    }

    pub fn enable_unsolicited(
        &self,
        classes: EventClasses,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            RequestDetails::EmptyResponse(BasicRequestImpl::new(
                BasicRequest::DisableUnsolicited(classes),
                handler,
            )),
        )
    }

    pub fn select_before_operate(
        &self,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            RequestDetails::Command(CommandRequestImpl::select_before_operate(headers, handler)),
        )
    }

    pub fn direct_operate(
        &self,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            RequestDetails::Command(CommandRequestImpl::direct_operate(headers, handler)),
        )
    }
}
