use crate::app::header::{ResponseHeader, IIN};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::handlers::{ReadTaskHandler, SessionHandler};
use crate::master::poll::Poll;
use crate::master::request::MasterRequest;
use crate::master::requests::auto::AutoRequestDetails;
use crate::master::requests::command::CommandRequestDetails;
use crate::master::requests::read::ReadRequestDetails;
use crate::master::types::{
    AutoRequest, CommandHeader, CommandTaskHandler, EventClasses, ReadRequest,
};
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

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

#[derive(Clone)]
pub(crate) struct TaskStates {
    disable_unsolicited: std::cell::Cell<AutoTaskState>,
    clear_restart_iin: std::cell::Cell<AutoTaskState>,
    integrity_scan: std::cell::Cell<AutoTaskState>,
    enabled_unsolicited: std::cell::Cell<AutoTaskState>,
}

impl TaskStates {
    pub(crate) fn new() -> Self {
        Self {
            disable_unsolicited: std::cell::Cell::new(AutoTaskState::Pending),
            clear_restart_iin: std::cell::Cell::new(AutoTaskState::Idle),
            integrity_scan: std::cell::Cell::new(AutoTaskState::Pending),
            enabled_unsolicited: std::cell::Cell::new(AutoTaskState::Pending),
        }
    }
}

struct Shared {
    address: u16,
    seq: std::cell::Cell<Sequence>,
    tasks: TaskStates,
    handler: std::cell::RefCell<Box<dyn SessionHandler>>,
}

#[derive(Clone)]
pub(crate) struct SessionHandle {
    inner: std::rc::Rc<Shared>,
}

#[derive(Clone)]
pub struct Session {
    config: SessionConfig,
    handle: SessionHandle,
    polls: Vec<Poll>,
}

impl Shared {
    fn new(address: u16, handler: Box<dyn SessionHandler>) -> Self {
        Self {
            address,
            seq: std::cell::Cell::new(Sequence::default()),
            tasks: TaskStates::new(),
            handler: std::cell::RefCell::new(handler),
        }
    }
}

impl SessionHandle {
    pub(crate) fn address(&self) -> u16 {
        self.inner.address
    }

    pub(crate) fn increment_seq(&self) -> Sequence {
        let value = self.inner.seq.get();
        self.inner.seq.set(Sequence::new(value.next()));
        value
    }

    pub(crate) fn previous_seq(&self) -> u8 {
        self.inner.seq.get().previous()
    }

    pub(crate) fn process_response_iin(&mut self, iin: IIN) {
        if iin.iin1.get_device_restart() {
            self.on_restart_iin_observed()
        }
    }

    pub(crate) fn on_restart_iin_observed(&self) {
        if let AutoTaskState::Idle = self.inner.tasks.clear_restart_iin.get() {
            log::warn!(
                "device restart detected (address == {})",
                self.inner.address
            );
            self.inner
                .tasks
                .clear_restart_iin
                .set(AutoTaskState::Pending);
        }
    }

    pub(crate) fn on_integrity_scan_complete(&self) {
        self.inner.tasks.integrity_scan.set(AutoTaskState::Idle);
    }

    pub(crate) fn on_clear_restart_iin_response(&self, iin: IIN) {
        if iin.iin1.get_device_restart() {
            self.inner
                .tasks
                .clear_restart_iin
                .set(AutoTaskState::Failed);
        } else {
            self.inner.tasks.clear_restart_iin.set(AutoTaskState::Idle);
        }
    }

    pub(crate) fn on_enable_unsolicited_response(&self, _iin: IIN) {
        self.inner
            .tasks
            .enabled_unsolicited
            .set(AutoTaskState::Idle);
    }

    pub(crate) fn on_disable_unsolicited_response(&self, _iin: IIN) {
        self.inner
            .tasks
            .disable_unsolicited
            .set(AutoTaskState::Idle);
    }

    pub(crate) fn handle_response(&self, header: ResponseHeader, objects: HeaderCollection) {
        self.inner
            .handler
            .borrow_mut()
            .handle(self.address(), header, objects)
    }
}

impl Session {
    pub fn new(address: u16, config: SessionConfig, handler: Box<dyn SessionHandler>) -> Self {
        Self {
            config,
            handle: SessionHandle {
                inner: std::rc::Rc::new(Shared::new(address, handler)),
            },
            polls: Vec::new(),
        }
    }

    pub fn add_poll(&mut self, request: ReadRequest, period: Duration) {
        self.polls.push(Poll::new(request, period));
    }

    pub fn next_request(&self) -> Option<MasterRequest> {
        if self.handle.inner.tasks.clear_restart_iin.get().is_pending() {
            return Some(self.clear_restart_iin());
        }
        if self.config.disable_unsol_classes.any()
            && self
                .handle
                .inner
                .tasks
                .disable_unsolicited
                .get()
                .is_pending()
        {
            return Some(self.disable_unsolicited(self.config.disable_unsol_classes));
        }
        if self.handle.inner.tasks.integrity_scan.get().is_pending() {
            return Some(self.integrity());
        }
        if self.config.enable_unsol_classes.any()
            && self
                .handle
                .inner
                .tasks
                .enabled_unsolicited
                .get()
                .is_pending()
        {
            return Some(self.enable_unsolicited(self.config.enable_unsol_classes));
        }
        None
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

    pub(crate) fn get(&mut self, address: u16) -> Option<SessionHandle> {
        self.sessions.get(&address).map(|x| x.handle.clone())
    }

    pub(crate) fn next_task(&mut self) -> Option<MasterRequest> {
        // don't try to rotate the tasks more times than the length of the queue
        for _ in 0..self.priority.len() {
            if let Some(session) = self.priority.front() {
                match session.next_request() {
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

// helpers to produce request tasks
impl Session {
    pub fn read(&self, request: ReadRequest, handler: Box<dyn ReadTaskHandler>) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            ReadRequestDetails::create(request, handler),
        )
    }

    fn clear_restart_iin(&self) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            AutoRequestDetails::create(AutoRequest::ClearRestartBit),
        )
    }

    fn integrity(&self) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            AutoRequestDetails::create(AutoRequest::IntegrityScan),
        )
    }

    fn disable_unsolicited(&self, classes: EventClasses) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            AutoRequestDetails::create(AutoRequest::DisableUnsolicited(classes)),
        )
    }

    fn enable_unsolicited(&self, classes: EventClasses) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            AutoRequestDetails::create(AutoRequest::EnableUnsolicited(classes)),
        )
    }

    pub fn select_before_operate(
        &self,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            CommandRequestDetails::select_before_operate(headers, handler),
        )
    }

    pub fn direct_operate(
        &self,
        headers: Vec<CommandHeader>,
        handler: Box<dyn CommandTaskHandler>,
    ) -> MasterRequest {
        MasterRequest::new(
            self.handle.clone(),
            CommandRequestDetails::direct_operate(headers, handler),
        )
    }
}
