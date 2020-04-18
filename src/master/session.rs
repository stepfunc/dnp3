use crate::app::header::{ResponseHeader, IIN};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, SessionHandler};
use crate::master::request::MasterRequest;
use crate::master::requests::auto::AutoRequestDetails;
use crate::master::requests::command::CommandRequestDetails;
use crate::master::requests::read::ReadRequestDetails;
use crate::master::types::{
    AutoRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};
use std::collections::{BTreeMap, VecDeque};

#[derive(Copy, Clone)]
pub struct SessionConfig {
    /// The event classes to disable on startup
    pub(crate) disable_unsol_classes: EventClasses,
    /// The event classes to enable on startup
    pub(crate) enable_unsol_classes: EventClasses,
}

impl SessionConfig {
    pub fn new(disable_unsol_classes: EventClasses, enable_unsol_classes: EventClasses) -> Self {
        Self {
            disable_unsol_classes,
            enable_unsol_classes,
        }
    }

    pub fn none() -> Self {
        SessionConfig::new(EventClasses::none(), EventClasses::none())
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfig::new(EventClasses::all(), EventClasses::all())
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) enum AutoTaskState {
    /// The task doesn't need to run
    Idle,
    /// The task needs to run
    Pending,
    /// The task has permanently failed
    Failed,
}

impl AutoTaskState {
    pub(crate) fn is_pending(self) -> bool {
        match self {
            AutoTaskState::Pending => true,
            _ => false,
        }
    }
}

pub(crate) struct TaskStates {
    disable_unsolicited: AutoTaskState,
    clear_restart_iin: AutoTaskState,
    integrity_scan: AutoTaskState,
    enabled_unsolicited: AutoTaskState,
}

impl TaskStates {
    pub(crate) fn new() -> Self {
        Self {
            disable_unsolicited: AutoTaskState::Pending,
            clear_restart_iin: AutoTaskState::Idle,
            integrity_scan: AutoTaskState::Pending,
            enabled_unsolicited: AutoTaskState::Pending,
        }
    }
}

struct Shared {
    address: u16,
    config: SessionConfig,
    seq: Sequence,
    tasks: TaskStates,
    handler: Box<dyn SessionHandler>,
}

impl Shared {
    pub(crate) fn new(
        address: u16,
        config: SessionConfig,
        handler: Box<dyn SessionHandler>,
    ) -> Self {
        Self {
            address,
            config,
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
    pub fn new(destination: u16, config: SessionConfig, handler: Box<dyn SessionHandler>) -> Self {
        Self {
            shared: std::rc::Rc::new(std::cell::RefCell::new(Shared::new(
                destination,
                config,
                handler,
            ))),
        }
    }

    pub fn address(&self) -> u16 {
        self.shared.borrow().address
    }

    pub fn next_auto_request(&self) -> Option<MasterRequest> {
        let inner = self.shared.borrow();
        if inner.tasks.clear_restart_iin.is_pending() {
            return Some(self.clear_restart_iin());
        }
        if inner.config.disable_unsol_classes.any() && inner.tasks.disable_unsolicited.is_pending()
        {
            return Some(self.disable_unsolicited(inner.config.disable_unsol_classes));
        }
        if inner.tasks.integrity_scan.is_pending() {
            return Some(self.integrity());
        }
        if inner.config.enable_unsol_classes.any() && inner.tasks.enabled_unsolicited.is_pending() {
            return Some(self.enable_unsolicited(inner.config.enable_unsol_classes));
        }
        None
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

pub struct SessionMap {
    sessions: BTreeMap<u16, Session>,
    priority: VecDeque<Session>,
}

impl SessionMap {
    pub fn new() -> Self {
        Self {
            sessions: BTreeMap::new(),
            priority: VecDeque::new(),
        }
    }

    pub fn register(
        &mut self,
        address: u16,
        config: SessionConfig,
        handler: Box<dyn SessionHandler>,
    ) -> bool {
        if self.sessions.contains_key(&address) {
            return false;
        }

        let session = Session::new(address, config, handler);
        self.sessions.insert(address, session.clone());
        self.priority.push_front(session.clone());
        true
    }

    pub(crate) fn get(&mut self, address: u16) -> Option<&mut Session> {
        self.sessions.get_mut(&address)
    }

    pub(crate) fn next_task(&mut self) -> Option<MasterRequest> {
        // don't try to rotate the tasks more times than the length of the queue
        for _ in 0..self.priority.len() {
            if let Some(session) = self.priority.front() {
                match session.next_auto_request() {
                    Some(task) => return Some(task),
                    None => {
                        // move the current front to the back
                        self.priority.rotate_left(1);
                    }
                }
            }
        }

        None
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
            log::warn!("device restart detected (address == {})", shared.address);
            shared.tasks.clear_restart_iin = AutoTaskState::Pending;
        }
    }

    pub(crate) fn on_integrity_scan_response(&mut self, header: ResponseHeader) {
        if header.control.fin {
            self.shared.borrow_mut().tasks.integrity_scan = AutoTaskState::Idle;
        }
    }

    pub(crate) fn on_clear_restart_iin_response(&mut self, iin: IIN) {
        if iin.iin1.get_device_restart() {
            self.shared.borrow_mut().tasks.clear_restart_iin = AutoTaskState::Failed;
        } else {
            self.shared.borrow_mut().tasks.clear_restart_iin = AutoTaskState::Idle;
        }
    }

    pub(crate) fn on_enable_unsolicited_response(&mut self, _iin: IIN) {
        self.shared.borrow_mut().tasks.enabled_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn on_disable_unsolicited_response(&mut self, _iin: IIN) {
        self.shared.borrow_mut().tasks.disable_unsolicited = AutoTaskState::Idle;
    }
}

// helpers to produce request tasks
impl Session {
    pub fn read(&self, request: ReadRequest, handler: Box<dyn ReadTaskHandler>) -> MasterRequest {
        MasterRequest::new(self.clone(), ReadRequestDetails::create(request, handler))
    }

    fn clear_restart_iin(&self) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            AutoRequestDetails::create(AutoRequest::ClearRestartBit),
        )
    }

    fn integrity(&self) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            AutoRequestDetails::create(AutoRequest::IntegrityScan),
        )
    }

    fn disable_unsolicited(&self, classes: EventClasses) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            AutoRequestDetails::create(AutoRequest::DisableUnsolicited(classes)),
        )
    }

    fn enable_unsolicited(&self, classes: EventClasses) -> MasterRequest {
        MasterRequest::new(
            self.clone(),
            AutoRequestDetails::create(AutoRequest::EnableUnsolicited(classes)),
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
