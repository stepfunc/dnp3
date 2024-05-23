use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use xxhash_rust::xxh64::xxh64;

use crate::app::parse::parser::{HeaderCollection, Response};
use crate::app::Timestamp;
use crate::app::{ExponentialBackOff, FunctionCode, RetryStrategy};
use crate::app::{Iin, ResponseHeader};
use crate::app::{Sequence, Timeout};
use crate::link::EndpointAddress;
use crate::master::error::{AssociationError, TaskError, TimeSyncError};
use crate::master::extract::extract_measurements;
use crate::master::handler::AssociationHandler;
use crate::master::messages::AssociationMsgType;
use crate::master::poll::{PollHandle, PollMap, PollMsg};
use crate::master::request::{Classes, EventClasses, TimeSyncProcedure};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::time::TimeSyncTask;
use crate::master::tasks::NonReadTask::TimeSync;
use crate::master::tasks::{AppTask, AssociationTask, ReadTask, Task};
use crate::master::{AssociationInformation, ReadHandler, ReadType, TaskType};
use crate::util::Smallest;

use crate::master::promise::Promise;
use crate::transport::FragmentAddr;
use crate::util::session::RunError;
use tokio::time::Instant;

/// Configuration for a master association
#[derive(Debug, Copy, Clone)]
#[cfg_attr(
    feature = "serialization",
    derive(serde::Serialize, serde::Deserialize)
)]
pub struct AssociationConfig {
    /// timeout for responses on this association
    #[cfg_attr(feature = "serialization", serde(default))]
    pub response_timeout: Timeout,
    /// The event classes to disable on startup
    #[cfg_attr(feature = "serialization", serde(default = "EventClasses::all"))]
    pub disable_unsol_classes: EventClasses,
    /// The event classes to enable on startup
    #[cfg_attr(feature = "serialization", serde(default = "EventClasses::all"))]
    pub enable_unsol_classes: EventClasses,
    /// Startup integrity classes to ask on master startup and when an outstation restart is detected.
    ///
    /// For conformance, this should be Class 1230.
    #[cfg_attr(feature = "serialization", serde(default = "Classes::all"))]
    pub startup_integrity_classes: Classes,
    /// automatic time synchronization based on NEED_TIME IIN bit
    #[cfg_attr(feature = "serialization", serde(default))]
    pub auto_time_sync: Option<TimeSyncProcedure>,
    /// automatic tasks retry strategy
    #[cfg_attr(feature = "serialization", serde(default))]
    pub auto_tasks_retry_strategy: RetryStrategy,
    /// Keep-alive timeout
    ///
    /// When no bytes are received within this timeout value,
    /// a `REQUEST_LINK_STATUS` request is sent
    #[cfg_attr(feature = "serialization", serde(default))]
    pub keep_alive_timeout: Option<Duration>,
    /// Automatic integrity scan when a `EVENT_BUFFER_OVERFLOW` is detected
    #[cfg_attr(feature = "serialization", serde(default))]
    pub auto_integrity_scan_on_buffer_overflow: bool,
    /// Classes to perform an automatic class scan when their IIN bit is detected
    #[cfg_attr(feature = "serialization", serde(default = "EventClasses::none"))]
    pub event_scan_on_events_available: EventClasses,
    /// The maximum number of user requests (e.g. commands, adhoc reads, etc) that will be queued
    /// before back-pressure is applied by failing requests with TaskError::TooManyRequests
    #[cfg_attr(
        feature = "serialization",
        serde(default = "AssociationConfig::default_max_queued_user_requests")
    )]
    pub max_queued_user_requests: usize,
}

impl AssociationConfig {
    const fn default_max_queued_user_requests() -> usize {
        16
    }

    /// Construct an `AssociationConfig` specifying the unsolicited, integrity, and auto event scan behaviors
    ///
    /// Other fields are set to defaults
    pub fn new(
        disable_unsol_classes: EventClasses,
        enable_unsol_classes: EventClasses,
        startup_integrity_classes: Classes,
        event_scan_on_events_available: EventClasses,
    ) -> Self {
        Self {
            response_timeout: Timeout::default(),
            disable_unsol_classes,
            enable_unsol_classes,
            startup_integrity_classes,
            auto_time_sync: None,
            auto_tasks_retry_strategy: RetryStrategy::default(),
            keep_alive_timeout: None,
            auto_integrity_scan_on_buffer_overflow: false,
            event_scan_on_events_available,
            max_queued_user_requests: Self::default_max_queued_user_requests(),
        }
    }

    /// Construct an `AssociationConfig` which will not perform any of the default handshaking
    /// at the beginning of the communications session.
    pub fn quiet() -> Self {
        Self {
            response_timeout: Timeout::default(),
            disable_unsol_classes: EventClasses::none(),
            enable_unsol_classes: EventClasses::none(),
            startup_integrity_classes: Classes::none(),
            auto_time_sync: None,
            auto_tasks_retry_strategy: RetryStrategy::default(),
            keep_alive_timeout: None,
            auto_integrity_scan_on_buffer_overflow: false,
            event_scan_on_events_available: EventClasses::none(),
            max_queued_user_requests: Self::default_max_queued_user_requests(),
        }
    }
}

impl Default for AssociationConfig {
    fn default() -> Self {
        Self {
            response_timeout: Timeout::default(),
            disable_unsol_classes: EventClasses::all(),
            enable_unsol_classes: EventClasses::all(),
            startup_integrity_classes: Classes::all(),
            auto_time_sync: None,
            auto_tasks_retry_strategy: RetryStrategy::default(),
            keep_alive_timeout: None,
            auto_integrity_scan_on_buffer_overflow: true,
            event_scan_on_events_available: EventClasses::none(),
            max_queued_user_requests: Self::default_max_queued_user_requests(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum AutoTaskState {
    /// The task doesn't need to be scheduled (because it was completed or not required)
    Idle,
    /// The task needs to run
    Pending,
    /// The task failed and is waiting for retry
    Failed(ExponentialBackOff, Instant),
}

impl AutoTaskState {
    fn is_idle(&self) -> bool {
        matches!(self, Self::Idle)
    }

    /// Check if task is pending for execution or waiting
    fn is_pending(&self) -> bool {
        !self.is_idle()
    }

    /// Create a `Next<Task>`
    fn create_next_task(&self, builder: impl FnOnce() -> Task) -> Next<Task> {
        match self {
            Self::Idle => Next::None,
            Self::Pending => Next::Now(builder()),
            Self::Failed(_, next) => {
                if Instant::now() >= *next {
                    Next::Now(builder())
                } else {
                    Next::NotBefore(*next)
                }
            }
        }
    }

    /// Demand an execution of the task, returns true if the task was
    /// idle and this call changed the state to 'pending'
    fn demand(&mut self) -> bool {
        if self.is_idle() {
            *self = Self::Pending;
            true
        } else {
            false
        }
    }

    /// The task was accomplished
    fn done(&mut self) {
        *self = Self::Idle;
    }

    /// The task failed and needs rescheduling
    fn failure(&mut self, config: &AssociationConfig) {
        *self = match self {
            Self::Failed(backoff, _) => {
                let delay = backoff.on_failure();
                Self::Failed(backoff.clone(), Instant::now() + delay)
            }
            _ => {
                let mut backoff = ExponentialBackOff::new(config.auto_tasks_retry_strategy);
                let delay = backoff.on_failure();
                Self::Failed(backoff, Instant::now() + delay)
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct TaskStates {
    disable_unsolicited: AutoTaskState,
    integrity_scan: AutoTaskState,
    enabled_unsolicited: AutoTaskState,
    clear_restart_iin: AutoTaskState,
    time_sync: AutoTaskState,
    event_scan: AutoTaskState,
}

impl TaskStates {
    pub(crate) fn new() -> Self {
        Self {
            disable_unsolicited: AutoTaskState::Pending,
            integrity_scan: AutoTaskState::Pending,
            enabled_unsolicited: AutoTaskState::Pending,
            clear_restart_iin: AutoTaskState::Idle,
            time_sync: AutoTaskState::Idle,
            event_scan: AutoTaskState::Idle,
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::new();
    }

    fn on_restart_iin(&mut self) {
        self.clear_restart_iin.demand();
        self.integrity_scan.demand();
        self.enabled_unsolicited.demand();
    }

    fn next(&self, config: &AssociationConfig, association: &Association) -> Next<Task> {
        if self.clear_restart_iin.is_pending() {
            return self
                .clear_restart_iin
                .create_next_task(|| AutoTask::ClearRestartBit.wrap());
        }

        if config.disable_unsol_classes.any() && self.disable_unsolicited.is_pending() {
            return self.disable_unsolicited.create_next_task(|| {
                AutoTask::DisableUnsolicited(config.disable_unsol_classes).wrap()
            });
        }

        if config.startup_integrity_classes.any() && self.integrity_scan.is_pending() {
            return self.integrity_scan.create_next_task(|| {
                Task::App(AppTask::Read(ReadTask::StartupIntegrity(
                    config.startup_integrity_classes,
                )))
            });
        }

        if self.time_sync.is_pending() {
            if let Some(procedure) = config.auto_time_sync {
                return self.time_sync.create_next_task(|| {
                    TimeSync(TimeSyncTask::get_procedure(procedure, None)).wrap()
                });
            }
        }

        if config.enable_unsol_classes.any() && self.enabled_unsolicited.is_pending() {
            return self.enabled_unsolicited.create_next_task(|| {
                AutoTask::EnableUnsolicited(config.enable_unsol_classes).wrap()
            });
        }

        let events_to_scan = association.events_available & config.event_scan_on_events_available;
        if events_to_scan.any() {
            return self
                .event_scan
                .create_next_task(|| ReadTask::EventScan(events_to_scan).wrap());
        }

        Next::None
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct LastUnsolFragment {
    header: ResponseHeader,
    hash: u64,
}

impl LastUnsolFragment {
    fn new(response: &Response) -> Self {
        Self {
            header: response.header,
            hash: xxh64(response.raw_objects, 0),
        }
    }
}

/// A logical connection between a master and an outstation
/// as defined by the DNP3 standard. A master manages requests
/// and responses for multiple associations (i.e. multi-drop).
pub(crate) struct Association {
    address: FragmentAddr,
    response_timeout: Timeout,
    seq: Sequence,
    last_unsol_frag: Option<LastUnsolFragment>,
    request_queue: VecDeque<Task>,
    max_request_queue_size: usize,
    auto_tasks: TaskStates,
    read_handler: Box<dyn ReadHandler>,
    assoc_handler: Box<dyn AssociationHandler>,
    assoc_info: Box<dyn AssociationInformation>,
    config: AssociationConfig,
    polls: PollMap,
    next_link_status_deadline: Option<Instant>,
    startup_integrity_done: bool,
    events_available: EventClasses,
}

impl Association {
    pub(crate) fn new(
        address: FragmentAddr,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_info: Box<dyn AssociationInformation>,
    ) -> Self {
        let now = Instant::now();

        Self {
            response_timeout: config.response_timeout,
            address,
            seq: Sequence::default(),
            last_unsol_frag: None,
            request_queue: VecDeque::new(),
            max_request_queue_size: config.max_queued_user_requests,
            auto_tasks: TaskStates::new(),
            read_handler,
            assoc_handler,
            assoc_info,
            config,
            polls: PollMap::new(),
            next_link_status_deadline: config.keep_alive_timeout.map(|delay| now + delay),
            startup_integrity_done: false,
            events_available: EventClasses::none(),
        }
    }

    pub(crate) fn process_message(&mut self, msg: AssociationMsgType, is_connected: bool) {
        match msg {
            AssociationMsgType::QueueTask(task) => {
                if is_connected {
                    if self.request_queue.len() < self.max_request_queue_size {
                        self.request_queue.push_back(task);
                    } else {
                        task.on_task_error(Some(self), TaskError::TooManyRequests);
                    }
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
            task.on_task_error(Some(self), err.into());
        }

        // Reset the auto tasks
        self.auto_tasks.reset();
        self.startup_integrity_done = false;

        // Clear last unsolicited fragment
        self.last_unsol_frag = None;
    }

    pub(crate) fn get_system_time(&self) -> Option<Timestamp> {
        self.assoc_handler.get_current_time()
    }

    pub(crate) fn complete_poll(&mut self, id: u64) {
        self.polls.complete(id)
    }

    pub(crate) fn seq(&self) -> Sequence {
        self.seq
    }
    pub(crate) fn increment_seq(&mut self) -> Sequence {
        self.seq.increment()
    }

    pub(crate) fn is_integrity_complete(&self) -> bool {
        !self.config.startup_integrity_classes.any() || self.startup_integrity_done
    }

    pub(crate) fn process_iin(&mut self, iin: Iin) {
        if iin.iin1.get_device_restart() {
            self.on_restart_iin_observed()
        }
        if iin.iin1.get_need_time() {
            self.on_need_time_observed();
        }
        if iin.iin2.get_event_buffer_overflow() {
            self.on_event_buffer_overflow_observed();
        }

        // Check events
        self.events_available.class1 = iin.iin1.get_class_1_events();
        self.events_available.class2 = iin.iin1.get_class_2_events();
        self.events_available.class3 = iin.iin1.get_class_3_events();
        let classes_to_scan = self.events_available & self.config.event_scan_on_events_available;
        if classes_to_scan.any() && self.auto_tasks.event_scan.demand() {
            tracing::info!("scheduled auto event scan");
        }
    }

    pub(crate) fn on_restart_iin_observed(&mut self) {
        if self.auto_tasks.clear_restart_iin.is_idle() {
            tracing::warn!("device restart detected (address == {})", self.address.link);
            self.auto_tasks.on_restart_iin();
            self.startup_integrity_done = false;
        }
    }

    pub(crate) fn on_need_time_observed(&mut self) {
        self.auto_tasks.time_sync.demand();
    }

    pub(crate) fn on_event_buffer_overflow_observed(&mut self) {
        if self.config.auto_integrity_scan_on_buffer_overflow
            && self.auto_tasks.integrity_scan.demand()
        {
            tracing::info!("event buffer overflow detected, queuing integrity scan");
        }
    }

    pub(crate) fn on_integrity_scan_complete(&mut self) {
        self.auto_tasks.integrity_scan.done();
        self.startup_integrity_done = true;
    }

    pub(crate) fn on_integrity_scan_failure(&mut self) {
        tracing::warn!("startup integrity scan failed");
        self.auto_tasks.integrity_scan.failure(&self.config);
    }

    pub(crate) fn on_event_scan_complete(&mut self) {
        self.auto_tasks.event_scan.done();
    }

    pub(crate) fn on_event_scan_failure(&mut self) {
        tracing::warn!("automatic event scan failed");
        self.auto_tasks.event_scan.failure(&self.config);
    }

    pub(crate) fn on_clear_restart_iin_response(&mut self, iin: Iin) {
        if iin.iin1.get_device_restart() {
            tracing::warn!("device failed to clear restart IIN bit");
            self.auto_tasks.clear_restart_iin.failure(&self.config);
        } else {
            self.auto_tasks.clear_restart_iin.done();
        }
    }

    pub(crate) fn on_clear_restart_iin_failure(&mut self) {
        tracing::warn!("failed to clear restart IIN bit");
        self.auto_tasks.clear_restart_iin.failure(&self.config);
    }

    pub(crate) fn on_time_sync_success(&mut self) {
        self.auto_tasks.time_sync.done();
    }

    pub(crate) fn on_time_sync_failure(&mut self, err: TimeSyncError) {
        tracing::warn!("auto time sync failed: {}", err);
        self.auto_tasks.time_sync.failure(&self.config);
    }

    pub(crate) fn on_enable_unsolicited_response(&mut self, _iin: Iin) {
        self.auto_tasks.enabled_unsolicited.done();
    }

    pub(crate) fn on_enable_unsolicited_failure(&mut self) {
        tracing::warn!("failed to enable unsolicited responses");
        self.auto_tasks.enabled_unsolicited.failure(&self.config);
    }

    pub(crate) fn on_disable_unsolicited_response(&mut self, _iin: Iin) {
        self.auto_tasks.disable_unsolicited.done();
    }

    pub(crate) fn on_disable_unsolicited_failure(&mut self) {
        tracing::warn!("failed to disable unsolicited responses");
        self.auto_tasks.disable_unsolicited.failure(&self.config);
    }

    pub(crate) fn on_link_activity(&mut self) {
        self.next_link_status_deadline = self
            .config
            .keep_alive_timeout
            .map(|timeout| Instant::now() + timeout)
    }

    pub(crate) async fn handle_unsolicited_response(&mut self, response: &Response<'_>) -> bool {
        // Accept the fragment only if the startup sequence was completed or if it's a null response.
        //
        // Now here's the deal. According to TB2015-002a, we should also ignore null responses without
        // the DEVICE_RESTART (IIN1.7) bit set. But this creates a timeout race. Imagine you're a master
        // that disconnected from an outstation. When the master reconnects to the outstation, it will try
        // to perform an integrity poll, but the outstation might send an unsolicited null response without
        // IIN1.7 (cause it haven't restarted!). If the master ignores it, the outstation will wait until
        // the unsolicited confirm times out then send the deferred read response. This might elapse the
        // master timeout and here we go, we're now in the same situation as a video call with a 3 seconds
        // lag, each waiting for the other to talk, but end up talking at the same time.
        if self.is_integrity_complete() || response.raw_objects.is_empty() {
            // Update last fragment received
            let new_frag = LastUnsolFragment::new(response);
            let last_frag = self.last_unsol_frag.replace(new_frag);

            // Ignore repeat
            if last_frag == Some(new_frag) {
                tracing::warn!("ignoring duplicate unsolicited response");
                self.notify_unsolicited_response(true, new_frag.header.control.seq);
                return true; // still want to send confirmation if requested
            }

            if let Ok(objects) = response.objects {
                extract_measurements(
                    ReadType::Unsolicited,
                    response.header,
                    objects,
                    self.read_handler.as_mut(),
                )
                .await;
            }

            self.notify_unsolicited_response(false, new_frag.header.control.seq);

            true
        } else {
            tracing::warn!(
                "ignoring unsolicited response received before the end of the startup procedure"
            );
            false
        }
    }

    pub(crate) async fn handle_integrity_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
    ) {
        extract_measurements(
            ReadType::StartupIntegrity,
            header,
            objects,
            self.read_handler.as_mut(),
        )
        .await;
    }

    pub(crate) async fn handle_poll_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
    ) {
        extract_measurements(
            ReadType::PeriodicPoll,
            header,
            objects,
            self.read_handler.as_mut(),
        )
        .await;
    }

    pub(crate) async fn handle_event_scan_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
    ) {
        extract_measurements(
            ReadType::PeriodicPoll,
            header,
            objects,
            self.read_handler.as_mut(),
        )
        .await;
    }

    pub(crate) async fn handle_read_response(
        &mut self,
        header: ResponseHeader,
        objects: HeaderCollection<'_>,
    ) {
        extract_measurements(
            ReadType::SinglePoll,
            header,
            objects,
            self.read_handler.as_mut(),
        )
        .await;
    }

    pub(crate) fn notify_task_start(
        &mut self,
        task_type: TaskType,
        fc: FunctionCode,
        seq: Sequence,
    ) {
        self.assoc_info.task_start(task_type, fc, seq)
    }

    pub(crate) fn notify_task_success(
        &mut self,
        task_type: TaskType,
        fc: FunctionCode,
        seq: Sequence,
    ) {
        self.assoc_info.task_success(task_type, fc, seq);
    }

    pub(crate) fn notify_task_fail(&mut self, task_type: TaskType, err: TaskError) {
        self.assoc_info.task_fail(task_type, err);
    }

    fn notify_unsolicited_response(&mut self, is_duplicate: bool, seq: Sequence) {
        self.assoc_info.unsolicited_response(is_duplicate, seq);
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

    fn next_link_status_task(&self, now: Instant) -> Next<Task> {
        match self.next_link_status_deadline {
            None => Next::None,
            Some(next) => {
                if now >= next {
                    Next::Now(Task::LinkStatus(Promise::null()))
                } else {
                    Next::NotBefore(next)
                }
            }
        }
    }

    fn get_next_task(&self, now: Instant) -> Next<Task> {
        // Check for automatic tasks
        let next_auto_task = self.auto_tasks.next(&self.config, self);

        // Startup tasks must complete prior to polls to link status requests
        if !matches!(next_auto_task, Next::None) {
            return next_auto_task;
        }

        match self.polls.next(now) {
            Next::Now(poll) => {
                // always prioritize polls over link status requests
                Next::Now(Task::App(AppTask::Read(ReadTask::PeriodicPoll(poll))))
            }
            Next::NotBefore(next_poll) => {
                match self.next_link_status_task(now) {
                    Next::None => Next::NotBefore(next_poll),
                    Next::Now(x) => Next::Now(x),
                    Next::NotBefore(next_link_status) => {
                        // return the earlier one
                        Next::NotBefore(Instant::min(next_poll, next_link_status))
                    }
                }
            }
            Next::None => {
                // if there are no polls we just defer to the link status
                self.next_link_status_task(now)
            }
        }
    }
}

pub(crate) enum Next<T> {
    None,
    Now(T),
    NotBefore(Instant),
}

pub(crate) struct AssociationMap {
    map: BTreeMap<EndpointAddress, Association>,
    priority: VecDeque<EndpointAddress>,
}

impl Default for AssociationMap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Copy, Clone)]
pub(crate) struct NoAssociation {
    pub(crate) address: EndpointAddress,
}

impl AssociationMap {
    pub(crate) fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            priority: VecDeque::new(),
        }
    }

    pub(crate) fn get_timeout(&self, address: EndpointAddress) -> Result<Timeout, TaskError> {
        match self.map.get(&address) {
            Some(x) => Ok(x.response_timeout),
            None => Err(TaskError::NoSuchAssociation(address)),
        }
    }

    pub(crate) fn reset(&mut self, err: RunError) {
        for association in &mut self.map.values_mut() {
            association.reset(err);
        }
    }

    pub(crate) fn register(&mut self, session: Association) -> Result<(), AssociationError> {
        if self.map.contains_key(&session.address.link) {
            return Err(AssociationError::DuplicateAddress(session.address.link));
        }

        self.priority.push_back(session.address.link);
        self.map.insert(session.address.link, session);
        Ok(())
    }

    pub(crate) fn remove(&mut self, address: EndpointAddress) {
        self.map.remove(&address);
        self.priority.retain(|x| *x != address);
    }

    pub(crate) fn get_mut(
        &mut self,
        address: EndpointAddress,
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
