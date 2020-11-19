use crate::app::header::{ResponseHeader, IIN};
use crate::app::parse::parser::HeaderCollection;
use crate::app::sequence::Sequence;
use crate::app::types::Timestamp;
use crate::master::error::{AssociationError, TaskError, TimeSyncError};
use crate::master::extract::extract_measurements;
use crate::master::handle::{AssociationHandler, Promise};
use crate::master::messages::AssociationMsgType;
use crate::master::poll::{PollMap, PollMsg};
use crate::master::request::{EventClasses, TimeSyncProcedure};
use crate::master::session::RunError;
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::time::TimeSyncTask;
use crate::master::tasks::NonReadTask::TimeSync;
use crate::master::tasks::{AssociationTask, ReadTask, Task};
use crate::util::Smallest;
use std::collections::{BTreeMap, VecDeque};
use tokio::time::Instant;

use crate::entry::NormalAddress;
pub use crate::master::poll::PollHandle;

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
    pub(crate) fn is_idle(self) -> bool {
        matches!(self, AutoTaskState::Idle)
    }

    pub(crate) fn is_pending(self) -> bool {
        matches!(self, AutoTaskState::Pending)
    }

    pub(crate) fn set_pending_if_idle(&mut self) {
        if self.is_idle() {
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
pub(crate) struct Association {
    address: NormalAddress,
    seq: Sequence,
    request_queue: VecDeque<Task>,
    auto_tasks: TaskStates,
    handler: Box<dyn AssociationHandler>,
    config: Configuration,
    polls: PollMap,
}

impl Association {
    pub(crate) fn new(
        address: NormalAddress,
        config: Configuration,
        handler: Box<dyn AssociationHandler>,
    ) -> Self {
        Self {
            address,
            seq: Sequence::default(),
            request_queue: VecDeque::new(),
            auto_tasks: TaskStates::new(),
            handler,
            config,
            polls: PollMap::new(),
        }
    }

    pub(crate) fn process_message(&mut self, msg: AssociationMsgType, is_connected: bool) {
        match msg {
            AssociationMsgType::QueueTask(task) => {
                if is_connected {
                    self.request_queue.push_back(task);
                } else {
                    task.on_task_error(Some(self), TaskError::NoConnection);
                }
            }
            AssociationMsgType::Poll(msg) => {
                self.process_poll_message(msg);
            }
        }
    }

    fn process_poll_message(&mut self, msg: PollMsg) {
        match msg {
            PollMsg::AddPoll(association, request, period, callback) => {
                let id = self.polls.add(request, period);
                let handle = PollHandle::new(association, id);
                callback.complete(Ok(handle))
            }
            PollMsg::RemovePoll(id) => {
                self.polls.remove(id);
            }
            PollMsg::Demand(id) => {
                self.polls.demand(id);
            }
        }
    }

    fn reset(&mut self, err: RunError) {
        // Fail any pending requests
        while let Some(task) = self.request_queue.pop_front() {
            let task_err = match err {
                RunError::Shutdown => TaskError::Shutdown,
                RunError::Link(_) => TaskError::NoConnection,
            };
            task.on_task_error(Some(self), task_err);
        }

        // Reset the auto tasks
        self.auto_tasks.reset();
    }

    pub(crate) fn get_system_time(&self) -> Option<Timestamp> {
        self.handler.get_system_time()
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
            self.auto_tasks.time_sync.set_pending_if_idle();
        }
    }

    pub(crate) fn on_restart_iin_observed(&mut self) {
        if self.auto_tasks.clear_restart_iin.is_idle() {
            log::warn!("device restart detected (address == {})", self.address);
            self.auto_tasks.clear_restart_iin.set_pending_if_idle();
            // also redo the startup sequence
            self.auto_tasks.disable_unsolicited.set_pending_if_idle();
            self.auto_tasks.enabled_unsolicited.set_pending_if_idle();
            self.auto_tasks.integrity_scan.set_pending_if_idle();
        }
    }

    pub(crate) fn on_integrity_scan_complete(&mut self) {
        self.auto_tasks.integrity_scan = AutoTaskState::Idle;
    }

    pub(crate) fn on_integrity_scan_failure(&mut self) {
        log::warn!("startup integrity scan failed");
        self.auto_tasks.integrity_scan = AutoTaskState::Failed;
    }

    pub(crate) fn on_clear_restart_iin_response(&mut self, iin: IIN) {
        self.auto_tasks.clear_restart_iin = if iin.iin1.get_device_restart() {
            log::warn!("device failed to clear restart IIN bit");
            AutoTaskState::Failed
        } else {
            AutoTaskState::Idle
        }
    }

    pub(crate) fn on_clear_restart_iin_failure(&mut self) {
        log::warn!("device failed to clear restart IIN bit");
        self.auto_tasks.clear_restart_iin = AutoTaskState::Failed;
    }

    pub(crate) fn on_time_sync_success(&mut self) {
        self.auto_tasks.time_sync = AutoTaskState::Idle;
    }

    pub(crate) fn on_time_sync_failure(&mut self, err: TimeSyncError) {
        log::warn!("auto time sync failed: {}", err);
        self.auto_tasks.time_sync = AutoTaskState::Failed;
    }

    pub(crate) fn on_enable_unsolicited_response(&mut self, _iin: IIN) {
        self.auto_tasks.enabled_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn on_enable_unsolicited_failure(&mut self) {
        log::warn!("device failed to enable unsolicited responses");
        self.auto_tasks.enabled_unsolicited = AutoTaskState::Failed;
    }

    pub(crate) fn on_disable_unsolicited_response(&mut self, _iin: IIN) {
        self.auto_tasks.disable_unsolicited = AutoTaskState::Idle;
    }

    pub(crate) fn on_disable_unsolicited_failure(&mut self) {
        log::warn!("device failed to disable unsolicited responses");
        self.auto_tasks.disable_unsolicited = AutoTaskState::Failed;
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

    pub(crate) fn handle_read_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection,
    ) {
        // TODO: Get another poll handler?
        extract_measurements(header, objects, self.handler.get_default_poll_handler());
    }

    pub(crate) fn priority_task(&mut self) -> Option<Task> {
        while let Some(task) = self.request_queue.pop_front() {
            if let Some(task) = task.start(self) {
                return Some(task);
            }
        }

        None
    }

    fn next_task(&mut self, now: Instant) -> Next<Task> {
        loop {
            let next_task = self.get_next_task(now);

            if let Next::Now(task) = next_task {
                // Check if execution can still happen
                if let Some(task) = task.start(self) {
                    return Next::Now(task);
                }
            } else {
                return next_task;
            }
        }
    }

    fn get_next_task(&self, now: Instant) -> Next<Task> {
        // Check for automatic tasks
        if self.auto_tasks.clear_restart_iin.is_pending() {
            return Next::Now(AutoTask::ClearRestartBit.wrap());
        }
        if self.auto_tasks.time_sync.is_pending() {
            if let Some(procedure) = self.config.auto_time_sync {
                return Next::Now(
                    TimeSync(TimeSyncTask::get_procedure(procedure, Promise::None)).wrap(),
                );
            }
        }
        if self.config.disable_unsol_classes.any()
            && self.auto_tasks.disable_unsolicited.is_pending()
        {
            return Next::Now(
                AutoTask::DisableUnsolicited(self.config.disable_unsol_classes).wrap(),
            );
        }
        if self.auto_tasks.integrity_scan.is_pending() {
            return Next::Now(Task::Read(ReadTask::StartupIntegrity));
        }
        if self.config.enable_unsol_classes.any()
            && self.auto_tasks.enabled_unsolicited.is_pending()
        {
            return Next::Now(AutoTask::EnableUnsolicited(self.config.enable_unsol_classes).wrap());
        }

        // Check for polls
        match self.polls.next(now) {
            Next::None => Next::None,
            Next::NotBefore(x) => Next::NotBefore(x),
            Next::Now(poll) => Next::Now(Task::Read(ReadTask::PeriodicPoll(poll))),
        }
    }
}

pub(crate) enum Next<T> {
    None,
    Now(T),
    NotBefore(Instant),
}

pub(crate) struct AssociationMap {
    map: BTreeMap<NormalAddress, Association>,
    priority: VecDeque<NormalAddress>,
}

impl Default for AssociationMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct NoAssociation {
    pub(crate) address: NormalAddress,
}

impl AssociationMap {
    pub(crate) fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            priority: VecDeque::new(),
        }
    }

    pub(crate) fn reset(&mut self, err: RunError) {
        for association in &mut self.map.values_mut() {
            association.reset(err);
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

    pub(crate) fn remove(&mut self, address: NormalAddress) {
        self.map.remove(&address);
        self.priority.retain(|x| *x != address);
    }

    pub(crate) fn get(&self, address: NormalAddress) -> Result<&Association, NoAssociation> {
        match self.map.get(&address) {
            Some(x) => Ok(x),
            None => Err(NoAssociation { address }),
        }
    }

    pub(crate) fn get_mut(
        &mut self,
        address: NormalAddress,
    ) -> Result<&mut Association, NoAssociation> {
        match self.map.get_mut(&address) {
            Some(x) => Ok(x),
            None => Err(NoAssociation { address }),
        }
    }

    pub(crate) fn next_task(&mut self) -> Next<AssociationTask> {
        // Check for priority task
        for (index, address) in self.priority.iter().enumerate() {
            if let Some(association) = self.map.get_mut(address) {
                // Check for priority task
                if let Some(task) = association.priority_task() {
                    // just before returning, move this session to last priority
                    if let Some(x) = self.priority.remove(index) {
                        self.priority.push_back(x);
                    }

                    let task = AssociationTask::new(association.address, task);
                    return Next::Now(task);
                }
            }
        }

        // Check for non-priority tasks
        let now = Instant::now();
        let mut earliest = Smallest::<Instant>::new();

        for (index, address) in self.priority.iter().enumerate() {
            if let Some(association) = self.map.get_mut(address) {
                match association.next_task(now) {
                    Next::Now(task) => {
                        // just before returning, move this session to last priority
                        if let Some(x) = self.priority.remove(index) {
                            self.priority.push_back(x);
                        }

                        let task = AssociationTask::new(association.address, task);
                        return Next::Now(task);
                    }
                    Next::NotBefore(x) => earliest.observe(x),
                    Next::None => {}
                }
            }
        }

        // Return earliest task
        if let Some(x) = earliest.value() {
            return Next::NotBefore(x);
        }

        // No task found
        Next::None
    }
}
