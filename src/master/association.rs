use crate::app::header::{ResponseHeader, IIN};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::error::AssociationError;
use crate::master::extract::extract_measurements;
use crate::master::handle::{AssociationHandler, Promise};
use crate::master::poll::{Poll, PollMap};
use crate::master::request::{EventClasses, ReadRequest, TimeSyncProcedure};
use crate::master::task::NonReadTask::TimeSync;
use crate::master::task::{ReadTask, Task, TaskType};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::time::TimeSyncTask;
use crate::util::Smallest;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;
use tokio::time::Instant;

#[derive(Copy, Clone)]
pub struct Configuration {
    /// The event classes to disable on startup
    pub disable_unsol_classes: EventClasses,
    /// The event classes to enable on startup
    pub enable_unsol_classes: EventClasses,
    /// automatic time synchronization based on NEED_TIME IIN bit
    pub auto_time_sync: Option<TimeSyncProcedure>,
}

impl Configuration {
    pub fn new(
        disable_unsol_classes: EventClasses,
        enable_unsol_classes: EventClasses,
        auto_time_sync: Option<TimeSyncProcedure>,
    ) -> Self {
        Self {
            disable_unsol_classes,
            enable_unsol_classes,
            auto_time_sync,
        }
    }

    pub fn none() -> Self {
        Configuration::new(EventClasses::none(), EventClasses::none(), None)
    }
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration::new(EventClasses::all(), EventClasses::all(), None)
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

    pub(crate) fn set_pending_if_idle(&mut self) {
        if let AutoTaskState::Idle = self {
            *self = AutoTaskState::Pending;
        }
    }
}

#[derive(Clone)]
pub(crate) struct TaskStates {
    disable_unsolicited: AutoTaskState,
    clear_restart_iin: AutoTaskState,
    time_sync: AutoTaskState,
    integrity_scan: AutoTaskState,
    enabled_unsolicited: AutoTaskState,
}

impl TaskStates {
    pub(crate) fn new() -> Self {
        Self {
            disable_unsolicited: AutoTaskState::Pending,
            clear_restart_iin: AutoTaskState::Idle,
            time_sync: AutoTaskState::Idle,
            integrity_scan: AutoTaskState::Pending,
            enabled_unsolicited: AutoTaskState::Pending,
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }
}

/// A logical connection between a master and an outstation
/// as defined by the DNP3 standard. A master manages requests
/// and responses for multiple associations (i.e. multi-drop).
pub struct Association {
    address: u16,
    seq: Sequence,
    tasks: TaskStates,
    handler: Box<dyn AssociationHandler>,
    config: Configuration,
    polls: PollMap,
}

impl Association {
    /// Create a new association:
    /// * `address` is the DNP3 link-layer address of the outstation
    /// * `config` controls the behavior of the master for this outstation
    /// * `handler` is a callback trait invoked when events occur for this outstation
    pub fn new(address: u16, config: Configuration, handler: Box<dyn AssociationHandler>) -> Self {
        Self {
            address,
            seq: Sequence::default(),
            tasks: TaskStates::new(),
            handler,
            config,
            polls: PollMap::new(),
        }
    }

    /// Add a poll to the association
    /// * `request` defines what data is being requested
    /// * `period` defines how often the READ operation is performed
    pub fn add_poll(&mut self, request: ReadRequest, period: Duration) {
        self.polls.add(request, period)
    }

    pub(crate) fn get_system_time(&self) -> std::time::SystemTime {
        self.handler.get_system_time()
    }

    pub(crate) fn get_address(&self) -> u16 {
        self.address
    }

    pub(crate) fn complete_poll(&mut self, id: u64) {
        self.polls.complete(id)
    }

    pub(crate) fn increment_seq(&mut self) -> Sequence {
        self.seq.increment()
    }

    pub(crate) fn process_iin(&mut self, iin: IIN) {
        if iin.iin1.get_device_restart() {
            self.on_restart_iin_observed()
        }
        if iin.iin1.get_need_time() {
            self.tasks.time_sync.set_pending_if_idle();
        }
    }

    pub(crate) fn on_restart_iin_observed(&mut self) {
        if let AutoTaskState::Idle = self.tasks.clear_restart_iin {
            log::warn!("device restart detected (address == {})", self.address);
            self.tasks.clear_restart_iin = AutoTaskState::Pending;
            // also redo the startup sequence
            self.tasks.disable_unsolicited = AutoTaskState::Pending;
            self.tasks.enabled_unsolicited = AutoTaskState::Pending;
            self.tasks.integrity_scan = AutoTaskState::Pending;
        }
    }

    pub(crate) fn on_integrity_scan_complete(&mut self) {
        self.tasks.integrity_scan = AutoTaskState::Idle;
    }

    pub(crate) fn on_clear_restart_iin_response(&mut self, iin: IIN) {
        self.tasks.clear_restart_iin = if iin.iin1.get_device_restart() {
            log::warn!("device failed to clear restart IIN bit");
            AutoTaskState::Failed
        } else {
            AutoTaskState::Idle
        }
    }

    pub(crate) fn on_time_sync_iin_response(&mut self, iin: IIN) {
        self.tasks.time_sync = if iin.iin1.get_need_time() {
            log::warn!("device failed to clear NEED_TIME IIN bit");
            AutoTaskState::Failed
        } else {
            AutoTaskState::Idle
        }
    }

    pub(crate) fn on_enable_unsolicited_response(&mut self, _iin: IIN) {
        self.tasks.enabled_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn on_disable_unsolicited_response(&mut self, _iin: IIN) {
        self.tasks.disable_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn handle_unsolicited_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) {
        extract_measurements(header, objects, self.handler.get_unsolicited_handler());
    }

    pub(crate) fn handle_integrity_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) {
        extract_measurements(header, objects, self.handler.get_integrity_handler());
    }

    pub(crate) fn handle_poll_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) {
        extract_measurements(header, objects, self.handler.get_default_poll_handler());
    }

    pub(crate) fn next_request(&self, now: Instant) -> Next<Task> {
        if self.tasks.clear_restart_iin.is_pending() {
            return Next::Now(self.clear_restart_iin());
        }
        if self.tasks.time_sync.is_pending() {
            if let Some(x) = self.config.auto_time_sync {
                return Next::Now(self.time_sync(x));
            }
        }
        if self.config.disable_unsol_classes.any() && self.tasks.disable_unsolicited.is_pending() {
            return Next::Now(self.disable_unsolicited(self.config.disable_unsol_classes));
        }
        if self.tasks.integrity_scan.is_pending() {
            return Next::Now(self.integrity());
        }
        if self.config.enable_unsol_classes.any() && self.tasks.enabled_unsolicited.is_pending() {
            return Next::Now(self.enable_unsolicited(self.config.enable_unsol_classes));
        }

        match self.polls.next(now) {
            Next::None => Next::None,
            Next::NotBefore(x) => Next::NotBefore(x),
            Next::Now(x) => Next::Now(self.poll(x)),
        }
    }
}

pub(crate) enum Next<T> {
    None,
    Now(T),
    NotBefore(Instant),
}

pub(crate) struct AssociationMap {
    map: BTreeMap<u16, Association>,
    priority: VecDeque<u16>,
}

impl Default for AssociationMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct NoAssociation {
    pub(crate) address: u16,
}

impl AssociationMap {
    pub(crate) fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            priority: VecDeque::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        for session in &mut self.map.values_mut() {
            session.tasks.reset();
        }
    }

    pub(crate) fn register(&mut self, session: Association) -> Result<(), AssociationError> {
        if self.map.contains_key(&session.address) {
            return Err(AssociationError::DuplicateAddress(session.address));
        }

        self.priority.push_back(session.address);
        self.map.insert(session.address, session);
        Ok(())
    }

    pub(crate) fn remove(&mut self, address: u16) {
        self.map.remove(&address);
        self.priority.retain(|x| *x != address);
    }

    pub(crate) fn get(&mut self, address: u16) -> Result<&Association, NoAssociation> {
        match self.map.get(&address) {
            Some(x) => Ok(x),
            None => Err(NoAssociation { address }),
        }
    }

    pub(crate) fn get_mut(&mut self, address: u16) -> Result<&mut Association, NoAssociation> {
        match self.map.get_mut(&address) {
            Some(x) => Ok(x),
            None => Err(NoAssociation { address }),
        }
    }

    pub(crate) fn next_task(&mut self) -> Next<Task> {
        let now = Instant::now();

        let mut earliest = Smallest::<Instant>::new();

        // don't try to rotate the tasks more times than the length of the queue
        for index in 0..self.priority.len() {
            if let Some(address) = self.priority.get(index) {
                if let Some(session) = self.map.get(address) {
                    match session.next_request(now) {
                        Next::Now(request) => {
                            // just before returning, move this session to last priority
                            if let Some(x) = self.priority.remove(index) {
                                self.priority.push_back(x);
                            }
                            return Next::Now(request);
                        }
                        Next::NotBefore(x) => earliest.observe(x),
                        Next::None => {}
                    }
                }
            }
        }

        if let Some(x) = earliest.value() {
            return Next::NotBefore(x);
        }

        Next::None
    }
}

// helpers to produce request tasks
impl Association {
    fn poll(&self, poll: Poll) -> Task {
        Task::new(self.address, TaskType::Read(ReadTask::PeriodicPoll(poll)))
    }

    fn integrity(&self) -> Task {
        Task::new(self.address, TaskType::Read(ReadTask::StartupIntegrity))
    }

    fn clear_restart_iin(&self) -> Task {
        Task::new(self.address, AutoTask::ClearRestartBit.wrap())
    }

    fn disable_unsolicited(&self, classes: EventClasses) -> Task {
        Task::new(self.address, AutoTask::DisableUnsolicited(classes).wrap())
    }

    fn time_sync(&self, procedure: TimeSyncProcedure) -> Task {
        Task::new(
            self.address,
            TimeSync(TimeSyncTask::get_procedure(procedure, true, Promise::None)).wrap(),
        )
    }

    fn enable_unsolicited(&self, classes: EventClasses) -> Task {
        Task::new(self.address, AutoTask::EnableUnsolicited(classes).wrap())
    }
}
