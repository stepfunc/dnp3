use crate::app::header::{ResponseHeader, IIN};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, RequestCompletionHandler, SessionHandler};
use crate::master::request::MasterRequest;
use crate::master::requests::auto::AutoRequestDetails;
use crate::master::requests::command::CommandRequestDetails;
use crate::master::requests::read::ReadRequestDetails;
use crate::master::types::{
    AutoRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};
use std::collections::BTreeMap;

pub(crate) enum AutoTaskState {
    /// The task doesn't need to run
    Idle,
    /// The task needs to run
    Pending,
    /// The task has permanently failed
    Failed,
}

pub(crate) struct TaskStates {
    clear_restart_iin: AutoTaskState,
}

impl TaskStates {
    pub(crate) fn new() -> Self {
        Self {
            clear_restart_iin: AutoTaskState::Idle,
        }
    }
}

struct Shared {
    address: u16,
    seq: Sequence,
    tasks: TaskStates,
    handler: Box<dyn SessionHandler>,
}

impl Shared {
    pub(crate) fn new(address: u16, handler: Box<dyn SessionHandler>) -> Self {
        Self {
            address,
            seq: Sequence::default(),
            tasks: TaskStates::new(),
            handler,
        }
    }
}

#[derive(Clone)]
pub struct Session {
    shared: std::rc::Rc<std::cell::RefCell<Shared>>,
}

impl Session {
    pub fn new(destination: u16, handler: Box<dyn SessionHandler>) -> Self {
        Self {
            shared: std::rc::Rc::new(std::cell::RefCell::new(Shared::new(destination, handler))),
        }
    }

    pub fn address(&self) -> u16 {
        self.shared.borrow().address
    }

    pub fn handle_unsolicited(
        &mut self,
        source: u16,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) {
        self.shared
            .borrow_mut()
            .handler
            .handle(source, header, objects)
    }
}

pub(crate) struct SessionMap {
    sessions: BTreeMap<u16, Session>,
}

impl SessionMap {
    pub(crate) fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
        }
    }

    pub(crate) fn get(&mut self, address: u16) -> Option<&mut Session> {
        self.sessions.get_mut(&address)
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
        if let AutoTaskState::Idle = shared.tasks.clear_restart_iin {
            log::warn!("device restart detected (address == {}", shared.address);
            shared.tasks.clear_restart_iin = AutoTaskState::Pending;
        }
    }

    pub(crate) fn on_clear_restart_iin_failed(&mut self) {
        self.shared.borrow_mut().tasks.clear_restart_iin = AutoTaskState::Failed;
    }

    pub(crate) fn on_clear_restart_iin_success(&mut self) {
        self.shared.borrow_mut().tasks.clear_restart_iin = AutoTaskState::Idle;
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
