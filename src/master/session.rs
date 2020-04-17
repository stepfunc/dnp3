use crate::app::header::IIN;
use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, RequestCompletionHandler};
use crate::master::request::MasterRequest;
use crate::master::requests::auto::AutoRequestDetails;
use crate::master::requests::command::CommandRequestDetails;
use crate::master::requests::read::ReadRequestDetails;
use crate::master::types::{
    AutoRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};

pub(crate) enum RestartIINState {
    Cleared,
    Asserted,
    Failed,
}

struct Shared {
    destination: u16,
    seq: Sequence,
    restart: RestartIINState,
}

impl Shared {
    pub(crate) fn new(destination: u16) -> Self {
        Self {
            destination,
            seq: Sequence::default(),
            restart: RestartIINState::Cleared,
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
    pub(crate) fn increment_seq(&mut self) -> Sequence {
        self.shared.borrow_mut().seq.increment()
    }

    pub(crate) fn previous_seq(&self) -> u8 {
        self.shared.borrow().seq.previous()
    }

    pub(crate) fn process_response_iin(&mut self, iin: IIN) {
        if iin.iin1.get_device_restart() {
            self.on_restart_iin_observed()
        }
    }

    fn on_restart_iin_observed(&mut self) {
        let mut shared = self.shared.borrow_mut();
        if let RestartIINState::Cleared = shared.restart {
            shared.restart = RestartIINState::Asserted;
        }
    }

    pub(crate) fn on_clear_restart_iin_failed(&mut self) {
        self.shared.borrow_mut().restart = RestartIINState::Failed;
    }

    pub(crate) fn on_clear_restart_iin_success(&mut self) {
        self.shared.borrow_mut().restart = RestartIINState::Cleared;
    }
}

// helpers to produce request tasks
impl Session {
    pub fn read(&self, request: ReadRequest, handler: Box<dyn ReadTaskHandler>) -> MasterRequest {
        MasterRequest::new(self.clone(), ReadRequestDetails::create(request, handler))
    }

    pub fn disable_unsolicited(
        &self,
        classes: EventClasses,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            AutoRequestDetails::create(AutoRequest::DisableUnsolicited(classes), handler),
        )
    }

    pub fn enable_unsolicited(
        &self,
        classes: EventClasses,
        handler: Box<dyn RequestCompletionHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            AutoRequestDetails::create(AutoRequest::DisableUnsolicited(classes), handler),
        )
    }

    pub fn select_before_operate(
        &self,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            CommandRequestDetails::select_before_operate(headers, handler),
        )
    }

    pub fn direct_operate(
        &self,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            CommandRequestDetails::direct_operate(headers, handler),
        )
    }
}
