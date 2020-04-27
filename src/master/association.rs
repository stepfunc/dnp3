use crate::app::header::{ResponseHeader, IIN};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::master::error::AssociationError;
use crate::master::handle::{AssociationHandler, CommandResult, Promise};
use crate::master::poll::{Poll, PollMap};
use crate::master::task::{ReadTask, Task, TaskType};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::command::CommandTask;
use crate::master::types::{AutoRequest, CommandHeaders, CommandMode, EventClasses, ReadRequest};
use crate::util::Smallest;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

#[derive(Copy, Clone)]
pub struct AssociationConfig {
    /// The event classes to disable on startup
    pub(crate) disable_unsol_classes: EventClasses,
    /// The event classes to enable on startup
    pub(crate) enable_unsol_classes: EventClasses,
}

impl AssociationConfig {
    pub fn new(disable_unsol_classes: EventClasses, enable_unsol_classes: EventClasses) -> Self {
        Self {
            disable_unsol_classes,
            enable_unsol_classes,
        }
    }

    pub fn none() -> Self {
        AssociationConfig::new(EventClasses::none(), EventClasses::none())
    }
}

impl Default for AssociationConfig {
    fn default() -> Self {
        AssociationConfig::new(EventClasses::all(), EventClasses::all())
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

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }
}

pub struct Association {
    address: u16,
    seq: Sequence,
    tasks: TaskStates,
    handler: Box<dyn AssociationHandler>,
    config: AssociationConfig,
    polls: PollMap,
}

impl Association {
    pub fn new(
        address: u16,
        config: AssociationConfig,
        handler: Box<dyn AssociationHandler>,
    ) -> Self {
        Self {
            address,
            seq: Sequence::default(),
            tasks: TaskStates::new(),
            handler,
            config,
            polls: PollMap::new(),
        }
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

    pub(crate) fn on_enable_unsolicited_response(&mut self, _iin: IIN) {
        self.tasks.enabled_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn on_disable_unsolicited_response(&mut self, _iin: IIN) {
        self.tasks.disable_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn handle_response(&mut self, header: ResponseHeader, objects: HeaderCollection) {
        self.handler.handle(self.address, header, objects);
    }
}

pub(crate) enum Next<T> {
    None,
    Now(T),
    NotBefore(Instant),
}

impl Association {
    pub fn add_poll(&mut self, request: ReadRequest, period: Duration) {
        self.polls.add(request, period)
    }

    pub(crate) fn next_request(&self, now: Instant) -> Next<Task> {
        if self.tasks.clear_restart_iin.is_pending() {
            return Next::Now(self.clear_restart_iin());
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
        let now = std::time::Instant::now();

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
        Task::new(self.address, AutoTask::create(AutoRequest::ClearRestartBit))
    }

    fn disable_unsolicited(&self, classes: EventClasses) -> Task {
        Task::new(
            self.address,
            AutoTask::create(AutoRequest::DisableUnsolicited(classes)),
        )
    }

    fn enable_unsolicited(&self, classes: EventClasses) -> Task {
        Task::new(
            self.address,
            AutoTask::create(AutoRequest::EnableUnsolicited(classes)),
        )
    }

    pub(crate) fn operate(
        &self,
        mode: CommandMode,
        headers: CommandHeaders,
        promise: Promise<CommandResult>,
    ) -> Task {
        Task::new(self.address, CommandTask::operate(mode, headers, promise))
    }
}
