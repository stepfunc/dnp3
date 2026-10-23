// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

use xxhash_rust::xxh64::xxh64;

use crate::app::parse::parser::Response;
use crate::app::Timestamp;
use crate::app::{ExponentialBackOff, FunctionCode, RetryStrategy};
use crate::app::{Iin, ResponseHeader};
use crate::app::{Sequence, Timeout};
use crate::link::EndpointAddress;
use crate::master::error::{AssociationError, TaskError, TimeSyncError};
use crate::master::extract::{extract_measurements, Delivered, ProcessableResponse};
use crate::master::handler::AssociationHandler;
use crate::master::messages::AssociationMsgType;
use crate::master::poll::{Poll, PollHandle, PollMap, PollMsg};
#[cfg(feature = "unstable")]
use crate::master::request::ReadRequest;
use crate::master::request::{Classes, EventClasses, TimeSyncProcedure};
use crate::master::tasks::auto::AutoTask;
use crate::master::tasks::time::PendingTimeSync;
use crate::master::tasks::{AppTask, AssociationTask, QueuedTask, ReadTask, Task};
use crate::master::{AssociationInformation, ReadHandler, ReadType, TaskType};

use crate::master::promise::Promise;
use crate::transport::FragmentAddr;
use crate::util::session::RunError;
use tokio::time::Instant;

use crate::util::deadline::deadline;

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

    /// True if the task still has to run: now, or after a failure once its backoff ends
    fn is_active(&self) -> bool {
        !self.is_idle()
    }

    /// True if the task failed and is waiting to retry
    fn has_failed(&self) -> bool {
        matches!(self, Self::Failed(..))
    }

    /// Create a `Next<QueuedTask>`, given the time it is `now`
    fn create_next_task(
        &self,
        now: Instant,
        builder: impl FnOnce() -> QueuedTask,
    ) -> Next<QueuedTask> {
        match self {
            Self::Idle => Next::None,
            Self::Pending => Next::Now(builder()),
            Self::Failed(_, next) => {
                if now >= *next {
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

    /// The task was accomplished, but needs to run again
    fn again(&mut self) {
        *self = Self::Pending;
    }

    /// The task failed and needs rescheduling
    fn failure(&mut self, config: &AssociationConfig) {
        *self = match self {
            Self::Failed(backoff, _) => {
                let delay = backoff.on_failure();
                Self::Failed(backoff.clone(), deadline(Instant::now(), delay))
            }
            Self::Idle | Self::Pending => {
                let mut backoff = ExponentialBackOff::new(config.auto_tasks_retry_strategy);
                let delay = backoff.on_failure();
                Self::Failed(backoff, deadline(Instant::now(), delay))
            }
        }
    }
}

/// An automatic task that only starts once the outstation's restart is cleared
#[derive(Copy, Clone)]
enum AfterRestart {
    IntegrityScan,
    EnableUnsolicited,
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

    fn state_mut(&mut self, task: &AutoTask) -> &mut AutoTaskState {
        match task {
            AutoTask::ClearRestartBit => &mut self.clear_restart_iin,
            AutoTask::EnableUnsolicited(_) => &mut self.enabled_unsolicited,
            AutoTask::DisableUnsolicited(_) => &mut self.disable_unsolicited,
        }
    }

    fn on_restart_iin(&mut self) {
        self.clear_restart_iin.demand();
        self.integrity_scan.demand();
        self.enabled_unsolicited.demand();
    }

    /// Complete the integrity scan or `ENABLE_UNSOLICITED`, returning true if the run counts.
    ///
    /// Both only start once the outstation's restart is cleared, so if it reports a restart that
    /// isn't cleared yet when one of them completes, it restarted while the task ran: the run is
    /// void, and the task runs again as part of the post-restart sequence.
    fn complete_after_restart(&mut self, task: AfterRestart) -> bool {
        let restarted = self.clear_restart_iin.is_active();
        let state = match task {
            AfterRestart::IntegrityScan => &mut self.integrity_scan,
            AfterRestart::EnableUnsolicited => &mut self.enabled_unsolicited,
        };
        if restarted {
            state.again();
        } else {
            state.done();
        }
        !restarted
    }

    fn on_success(&mut self, task: &AutoTask) {
        match task {
            AutoTask::EnableUnsolicited(_) => {
                self.complete_after_restart(AfterRestart::EnableUnsolicited);
            }
            AutoTask::ClearRestartBit | AutoTask::DisableUnsolicited(_) => {
                self.state_mut(task).done()
            }
        }
    }

    fn next(
        &self,
        now: Instant,
        config: &AssociationConfig,
        association: &Association,
    ) -> Next<QueuedTask> {
        if self.clear_restart_iin.is_active() {
            return self
                .clear_restart_iin
                .create_next_task(now, || AutoTask::ClearRestartBit.wrap().into());
        }

        if config.disable_unsol_classes.any() && self.disable_unsolicited.is_active() {
            return self.disable_unsolicited.create_next_task(now, || {
                AutoTask::DisableUnsolicited(config.disable_unsol_classes)
                    .wrap()
                    .into()
            });
        }

        // the time sync goes before the integrity scan (IEEE 1815-2012 5.1.1.1.2 and 5.1.1.2),
        // unless it failed: then it retries after the scan once its backoff ends, and doesn't
        // hold up anything while it waits
        if !self.time_sync.has_failed() {
            if let Some(task) = self.time_sync_if_due(now, config) {
                return Next::Now(task);
            }
        }

        if config.startup_integrity_classes.any() && self.integrity_scan.is_active() {
            return self.integrity_scan.create_next_task(now, || {
                Task::App(AppTask::Read(ReadTask::StartupIntegrity(
                    config.startup_integrity_classes,
                )))
                .into()
            });
        }

        if let Some(task) = self.time_sync_if_due(now, config) {
            return Next::Now(task);
        }

        if config.enable_unsol_classes.any() && self.enabled_unsolicited.is_active() {
            return self.enabled_unsolicited.create_next_task(now, || {
                AutoTask::EnableUnsolicited(config.enable_unsol_classes)
                    .wrap()
                    .into()
            });
        }

        let events_to_scan = association.events_to_scan();
        if events_to_scan.any() {
            return self
                .event_scan
                .create_next_task(now, || ReadTask::EventScan(events_to_scan).wrap().into());
        }

        Next::None
    }

    /// The automatic time sync, if it's enabled and due now
    fn time_sync_if_due(&self, now: Instant, config: &AssociationConfig) -> Option<QueuedTask> {
        let procedure = config.auto_time_sync?;
        match self
            .time_sync
            .create_next_task(now, || PendingTimeSync::auto(procedure).into())
        {
            Next::Now(task) => Some(task),
            Next::NotBefore(_) | Next::None => None,
        }
    }

    /// When the automatic time sync retries, if it failed and is waiting for its backoff to end
    fn time_sync_retry(&self, now: Instant, config: &AssociationConfig) -> Option<Instant> {
        config.auto_time_sync?;
        match self.time_sync {
            AutoTaskState::Failed(_, next) if now < next => Some(next),
            AutoTaskState::Idle | AutoTaskState::Pending | AutoTaskState::Failed(..) => None,
        }
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
    seq: Sequence,
    last_unsol_frag: Option<LastUnsolFragment>,
    request_queue: VecDeque<QueuedTask>,
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

        Self::create(
            address,
            config,
            read_handler,
            assoc_handler,
            assoc_info,
            PollMap::new(),
            config.keep_alive_timeout.map(|delay| deadline(now, delay)),
        )
    }

    #[cfg(feature = "unstable")]
    pub(crate) fn new_deferred(
        address: FragmentAddr,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_info: Box<dyn AssociationInformation>,
    ) -> Self {
        Self::create(
            address,
            config,
            read_handler,
            assoc_handler,
            assoc_info,
            PollMap::new_deferred(),
            None,
        )
    }

    fn create(
        address: FragmentAddr,
        config: AssociationConfig,
        read_handler: Box<dyn ReadHandler>,
        assoc_handler: Box<dyn AssociationHandler>,
        assoc_info: Box<dyn AssociationInformation>,
        polls: PollMap,
        next_link_status_deadline: Option<Instant>,
    ) -> Self {
        Self {
            address,
            seq: Sequence::default(),
            last_unsol_frag: None,
            request_queue: VecDeque::new(),
            auto_tasks: TaskStates::new(),
            read_handler,
            assoc_handler,
            assoc_info,
            config,
            polls,
            next_link_status_deadline,
            startup_integrity_done: false,
            events_available: EventClasses::none(),
        }
    }

    #[cfg(feature = "unstable")]
    pub(crate) fn start(&mut self, now: Instant) {
        if self.polls.start(now) {
            self.next_link_status_deadline = self
                .config
                .keep_alive_timeout
                .map(|delay| deadline(now, delay));
        }
    }

    #[cfg(feature = "unstable")]
    pub(crate) fn add_poll(&mut self, request: ReadRequest, period: Duration) -> u64 {
        self.polls.add(request, period)
    }

    pub(crate) fn process_message(&mut self, msg: AssociationMsgType, is_connected: bool) {
        match msg {
            AssociationMsgType::QueueTask(task) => {
                if is_connected {
                    if self.request_queue.len() < self.config.max_queued_user_requests {
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

    /// Fail the queued requests of an association that is being removed
    fn fail_queued_requests(&mut self, err: TaskError) {
        while let Some(task) = self.request_queue.pop_front() {
            task.on_task_error(None, err);
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

    pub(crate) fn complete_poll(&mut self, run: &Poll) {
        self.polls.complete(run)
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
        if self.events_to_scan().any() && self.auto_tasks.event_scan.demand() {
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
        if self
            .auto_tasks
            .complete_after_restart(AfterRestart::IntegrityScan)
        {
            self.startup_integrity_done = true;
        }
    }

    pub(crate) fn on_integrity_scan_failure(&mut self) {
        self.auto_tasks.integrity_scan.failure(&self.config);
    }

    /// The classes with events available that the automatic event scan reads
    fn events_to_scan(&self) -> EventClasses {
        self.events_available & self.config.event_scan_on_events_available
    }

    /// Complete the event scan, given whether it returned events without an IIN2 request error
    pub(crate) fn on_event_scan_complete(&mut self, events_without_request_errors: bool) {
        // the scan's own response can report that more events remain. It only runs again at once
        // if it read some and the outstation didn't reject it, so an outstation that keeps
        // reporting events without returning any can't make the master scan back to back
        if events_without_request_errors && self.events_to_scan().any() {
            self.auto_tasks.event_scan.again();
        } else {
            self.auto_tasks.event_scan.done();
        }
    }

    pub(crate) fn on_event_scan_failure(&mut self) {
        self.auto_tasks.event_scan.failure(&self.config);
    }

    pub(crate) fn on_auto_task_success(&mut self, task: &AutoTask) {
        self.auto_tasks.on_success(task);
    }

    pub(crate) fn on_auto_task_failure(&mut self, task: &AutoTask) {
        self.auto_tasks.state_mut(task).failure(&self.config);
    }

    pub(crate) fn on_time_sync_success(&mut self) {
        self.auto_tasks.time_sync.done();
    }

    pub(crate) fn on_time_sync_failure(&mut self, err: TimeSyncError) {
        tracing::warn!("auto time sync failed: {}", err);
        self.auto_tasks.time_sync.failure(&self.config);
    }

    pub(crate) fn on_link_activity(&mut self) {
        self.next_link_status_deadline = self
            .config
            .keep_alive_timeout
            .map(|timeout| deadline(Instant::now(), timeout))
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
            let new_frag = LastUnsolFragment::new(response);

            // Ignore repeat
            if self.last_unsol_frag == Some(new_frag) {
                tracing::warn!("ignoring duplicate unsolicited response");
                self.notify_unsolicited_response(true, new_frag.header.control.seq);
                return true; // still want to send confirmation if requested
            }

            // An invalid response isn't confirmed or remembered, so the outstation's retry isn't
            // taken for a duplicate
            let objects = match ProcessableResponse::new(response.objects) {
                Ok(objects) => objects,
                Err(err) => {
                    tracing::warn!("discarding unsolicited response: {err}");
                    return false;
                }
            };

            self.last_unsol_frag = Some(new_frag);

            extract_measurements(
                ReadType::Unsolicited,
                response.header,
                objects,
                self.read_handler.as_mut(),
            )
            .await;

            self.notify_unsolicited_response(false, new_frag.header.control.seq);

            true
        } else {
            tracing::warn!(
                "ignoring unsolicited response received before the end of the startup procedure"
            );
            false
        }
    }

    pub(crate) async fn handle_read_response(
        &mut self,
        read_type: ReadType,
        header: ResponseHeader,
        objects: ProcessableResponse<'_>,
    ) -> Delivered {
        extract_measurements(read_type, header, objects, self.read_handler.as_mut()).await
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
            match self.get_next_task(now) {
                Next::Now(task) => {
                    // Check if execution can still happen
                    if let Some(task) = task.start(self) {
                        return Next::Now(task);
                    }
                }
                Next::NotBefore(time) => return Next::NotBefore(time),
                Next::None => return Next::None,
            }
        }
    }

    fn next_link_status_task(&self, now: Instant) -> Next<QueuedTask> {
        match self.next_link_status_deadline {
            None => Next::None,
            Some(next) => {
                if now >= next {
                    Next::Now(Task::LinkStatus(Promise::null()).into())
                } else {
                    Next::NotBefore(next)
                }
            }
        }
    }

    fn get_next_task(&self, now: Instant) -> Next<QueuedTask> {
        let next = self.get_next_scheduled_task(now);
        // a failed time sync waiting for its retry doesn't hold up the rest of the association
        match self.auto_tasks.time_sync_retry(now, &self.config) {
            Some(retry) => next.or_not_before(retry),
            None => next,
        }
    }

    fn get_next_scheduled_task(&self, now: Instant) -> Next<QueuedTask> {
        // Check for automatic tasks
        let next_auto_task = self.auto_tasks.next(now, &self.config, self);

        // Startup tasks must complete prior to polls to link status requests
        if !matches!(next_auto_task, Next::None) {
            return next_auto_task;
        }

        match self.polls.next(now) {
            Next::Now(poll) => {
                // always prioritize polls over link status requests
                Next::Now(Task::App(AppTask::Read(ReadTask::PeriodicPoll(poll))).into())
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

impl<T> Next<T> {
    /// Also wait for `deadline`, if nothing is ready now
    fn or_not_before(self, deadline: Instant) -> Self {
        match self {
            Self::Now(x) => Self::Now(x),
            Self::NotBefore(next) => Self::NotBefore(Instant::min(next, deadline)),
            Self::None => Self::NotBefore(deadline),
        }
    }
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

    #[cfg(feature = "unstable")]
    pub(crate) fn start(&mut self, now: Instant) {
        for association in self.map.values_mut() {
            association.start(now);
        }
    }

    pub(crate) fn get_timeout(&self, address: EndpointAddress) -> Result<Timeout, TaskError> {
        match self.map.get(&address) {
            Some(x) => Ok(x.config.response_timeout),
            None => Err(TaskError::NoSuchAssociation(address)),
        }
    }

    pub(crate) fn reset(&mut self, err: RunError) {
        for association in &mut self.map.values_mut() {
            association.reset(err);
        }
    }

    pub(crate) fn register(&mut self, session: Association) -> Result<(), AssociationError> {
        self.register_and_get(session).map(|_| ())
    }

    pub(crate) fn register_and_get(
        &mut self,
        session: Association,
    ) -> Result<&mut Association, AssociationError> {
        let address = session.address.link;
        match self.map.entry(address) {
            Entry::Occupied(_) => Err(AssociationError::DuplicateAddress(address)),
            Entry::Vacant(entry) => {
                self.priority.push_back(address);
                Ok(entry.insert(session))
            }
        }
    }

    pub(crate) fn remove(&mut self, address: EndpointAddress) {
        if let Some(mut association) = self.map.remove(&address) {
            association.fail_queued_requests(TaskError::NoSuchAssociation(address));
        }
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
        let mut earliest: Option<Instant> = None;

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
                    Next::NotBefore(x) => earliest = Some(earliest.map_or(x, |e| e.min(x))),
                    Next::None => {}
                }
            }
        }

        match earliest {
            Some(x) => Next::NotBefore(x),
            None => Next::None,
        }
    }
}

#[cfg(test)]
mod scheduler_tests;

#[cfg(test)]
mod retry_tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn retry_delays_too_long_to_represent_are_scheduled_far_in_the_future() {
        let config = AssociationConfig {
            auto_tasks_retry_strategy: RetryStrategy::new(Duration::MAX, Duration::MAX),
            ..AssociationConfig::default()
        };

        // the clock is paused, so the failures are recorded at this instant
        let now = Instant::now();
        let mut state = AutoTaskState::Pending;
        // the first failure, then a retry whose delay would double past the maximum
        for _ in 0..2 {
            state.failure(&config);
            assert!(matches!(
                state.create_next_task(now, unreachable_task),
                Next::NotBefore(x) if x == deadline(now, Duration::MAX)
            ));
        }
    }

    fn unreachable_task() -> QueuedTask {
        panic!("the task isn't due")
    }
}

#[cfg(test)]
#[cfg(feature = "unstable")]
mod timer_tests {
    use super::*;
    use crate::master::{AssociationHandler, AssociationInformation, ReadHandler};
    use crate::util::phys::PhysAddr;

    struct NullReadHandler;
    impl ReadHandler for NullReadHandler {}

    struct NullAssociationHandler;
    impl AssociationHandler for NullAssociationHandler {}

    struct NullAssociationInformation;
    impl AssociationInformation for NullAssociationInformation {}

    fn deferred_association(keep_alive_timeout: Duration) -> Association {
        let mut config = AssociationConfig::quiet();
        config.keep_alive_timeout = Some(keep_alive_timeout);
        Association::new_deferred(
            FragmentAddr {
                link: EndpointAddress::try_new(1).unwrap(),
                phys: PhysAddr::None,
            },
            config,
            Box::new(NullReadHandler),
            Box::new(NullAssociationHandler),
            Box::new(NullAssociationInformation),
        )
    }

    #[test]
    fn deferred_keep_alive_is_scheduled_from_start_time() {
        let timeout = Duration::from_secs(5);
        let start = Instant::now();
        let mut association = deferred_association(timeout);

        assert_eq!(association.next_link_status_deadline, None);
        association.start(start);
        assert_eq!(association.next_link_status_deadline, Some(start + timeout));
    }

    #[test]
    fn starting_twice_does_not_reset_keep_alive_deadline() {
        let timeout = Duration::from_secs(5);
        let start = Instant::now();
        let mut association = deferred_association(timeout);

        association.start(start);
        association.start(start + Duration::from_secs(1));
        assert_eq!(association.next_link_status_deadline, Some(start + timeout));
    }
}
