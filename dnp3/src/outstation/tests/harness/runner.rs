// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::control::CommandStatus;
use crate::decode::AppDecodeLevel;
use crate::link::header::{BroadcastConfirmMode, FrameInfo, FrameType};
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::EventBufferConfig;
use crate::outstation::session::task::OutstationTask;
use crate::outstation::tests::harness::{
    event_handlers, ApplicationData, ControlStatuses, Event, EventReceiver, MockControlHandler,
    MockOutstationApplication, MockOutstationInformation,
};
use crate::outstation::OutstationHandle;
use crate::phys::{PhysAddr, PhysLayer};
use crate::transport::mock::reader::RxFrameInfo;
use crate::transport::mock::MockTransport;
use crate::util::lifecycle::{Enabled, RunError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

pub(crate) fn get_default_config() -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.features.unsolicited = Feature::Disabled;
    config
}

pub(crate) fn get_default_unsolicited_config() -> OutstationConfig {
    let mut config = OutstationConfig::new(
        EndpointAddress::try_new(10).unwrap(),
        EndpointAddress::try_new(1).unwrap(),
        EventBufferConfig::all_types(5),
    );

    config.decode_level = AppDecodeLevel::ObjectValues.into();

    config
}

/// Type-state harness: `S` is the connection state and carries the per-state data, so
/// the connected/disconnected shapes are distinct types. IO methods exist only on
/// `Harness<Connected>`, so using them on a disconnected harness is a compile error
/// rather than a runtime panic.
pub(crate) type OutstationHarness = Harness<Connected>;

type MockOutstationTask = OutstationTask<MockTransport>;

type Session = JoinHandle<(MockOutstationTask, RunError)>;

pub(crate) struct Harness<S> {
    pub(crate) handle: OutstationHandle,
    state: S,
    events: EventReceiver,
    pub(crate) application_data: Arc<Mutex<ApplicationData>>,
    control_statuses: ControlStatuses,
    frame_info: RxFrameInfo,
    requests_from_idle: Arc<AtomicBool>,
}

/// connected state: a live mock IO handle and the running session
pub(crate) struct Connected {
    io: sfio_tokio_mock_io::Handle,
    session: Session,
}

/// disconnected state: the task (and its `DatabaseHandle`) parked between connections
pub(crate) struct Disconnected {
    task: MockOutstationTask,
}

async fn run_session(
    mut task: MockOutstationTask,
    mut io: PhysLayer,
) -> (MockOutstationTask, RunError) {
    let err = task.run(&mut io).await;
    (task, err)
}

fn spawn_session(task: MockOutstationTask) -> Connected {
    let (io, io_handle) = sfio_tokio_mock_io::mock();
    let phys = PhysLayer::mock(io);
    Connected {
        io: io_handle,
        session: tokio::spawn(run_session(task, phys)),
    }
}

// methods available in any connection state
impl<S> Harness<S> {
    pub(crate) async fn next_event(&mut self) -> Event {
        self.events.next().await
    }

    pub(crate) async fn wait_for_events(&mut self, expected: &[Event]) {
        for event in expected {
            let next = self.events.next().await;
            if next != *event {
                panic!("Expected {event:?} but next event is {next:?}");
            }
        }
    }

    pub(crate) fn check_events(&mut self, expected: &[Event]) {
        for event in expected {
            match self.events.poll() {
                Some(next) => {
                    if next != *event {
                        panic!("Expected {event:?} but next event is {next:?}")
                    }
                }
                None => panic!("Expected {event:?} but no event ready"),
            }
        }
    }

    pub(crate) fn poll_event(&mut self) -> Option<Event> {
        self.events.poll()
    }

    /// Every event already reported, without waiting for more
    pub(crate) fn drain_events(&mut self) -> Vec<Event> {
        std::iter::from_fn(|| self.events.poll()).collect()
    }

    pub(crate) fn check_no_events(&mut self) {
        if let Some(x) = self.events.poll() {
            panic!("expected no events, but next event is: {x:?}")
        }
    }

    /// Make the control handler return `status` for every control at `index`
    pub(crate) fn set_control_status(&self, index: u16, status: CommandStatus) {
        self.control_statuses.lock().unwrap().insert(index, status);
    }

    /// Report each `process_request_from_idle` call from now on as [`Event::RequestFromIdle`]
    pub(crate) fn record_requests_from_idle(&self) {
        self.requests_from_idle.store(true, Ordering::Relaxed);
    }

    /// Change the broadcast address of the requests sent from now on
    pub(crate) fn set_broadcast(&mut self, broadcast: Option<BroadcastConfirmMode>) {
        let mut info = self.frame_info.get().unwrap();
        info.broadcast = broadcast;
        self.frame_info.set(info);
    }
}

// methods that require a live connection
impl Harness<Connected> {
    pub(crate) async fn test_request_response(&mut self, request: &[u8], response: &[u8]) {
        self.send_and_process(request).await;
        self.expect_response(response).await;
    }

    pub(crate) async fn expect_write(&mut self) -> Vec<u8> {
        match self.state.io.next_event().await {
            sfio_tokio_mock_io::Event::Write(bytes) => bytes,
            x => panic!("Expected write but got: {x:?}"),
        }
    }

    pub(crate) async fn expect_response(&mut self, response: &[u8]) {
        assert_eq!(
            self.state.io.next_event().await,
            sfio_tokio_mock_io::Event::Write(response.to_vec())
        );
    }

    /// The next response the outstation already sent, if any, without waiting for one
    ///
    /// After [`Self::send_and_process`] returns, the outstation has sent everything it sends in
    /// answer to the request, so popping until this returns `None` sees every response to it. The
    /// `Event::Read` that `send_and_process` waits for only says that the outstation consumed the
    /// request's bytes, but the outstation can't have stopped short of answering when the test
    /// sees it:
    /// - the test runs on a current-thread runtime, so the test task only runs again once the
    ///   outstation task's poll returns;
    /// - the mock's read and write callbacks run inside that poll, and its writes are ready at
    ///   once, so the poll returns only when the outstation waits for its next input or timer.
    ///
    /// That holds while nothing the outstation awaits in between returns pending, which the mock
    /// application's deferred attribute writes (`defer_attr_writes`) do.
    pub(crate) fn pop_response(&mut self) -> Option<Vec<u8>> {
        match self.state.io.pop_event()? {
            sfio_tokio_mock_io::Event::Write(bytes) => Some(bytes),
            x => panic!("Expected write but got: {x:?}"),
        }
    }

    /// Every response the outstation already sent, as [`Self::pop_response`] describes
    pub(crate) fn drain_responses(&mut self) -> Vec<Vec<u8>> {
        std::iter::from_fn(|| self.pop_response()).collect()
    }

    pub(crate) fn expect_no_response(&mut self) {
        assert_eq!(self.state.io.pop_event(), None);
    }

    pub(crate) async fn send_and_process(&mut self, request: &[u8]) {
        self.state.io.read(request);
        assert_eq!(
            self.state.io.next_event().await,
            sfio_tokio_mock_io::Event::Read
        );
    }

    /// Drop the link via an injected read error and recover the task, asserting the
    /// session terminated with a `RunError::Link`. The task (and its `DatabaseHandle`)
    /// survive into the returned disconnected harness.
    pub(crate) async fn drop_link(self) -> Harness<Disconnected> {
        let Harness {
            handle,
            state: Connected { mut io, session },
            events,
            application_data,
            control_statuses,
            frame_info,
            requests_from_idle,
        } = self;

        io.read_error(std::io::ErrorKind::ConnectionReset);
        let (task, err) = session.await.expect("outstation task panicked");
        assert!(
            matches!(err, RunError::Link(_)),
            "expected RunError::Link but got {err:?}"
        );

        Harness {
            handle,
            state: Disconnected { task },
            events,
            application_data,
            control_statuses,
            frame_info,
            requests_from_idle,
        }
    }
}

impl Harness<Disconnected> {
    /// Start a new connection reusing the same task + database (as the real
    /// TCP/serial transports do across reconnects)
    pub(crate) fn reconnect(self) -> OutstationHarness {
        Harness {
            handle: self.handle,
            state: spawn_session(self.state.task),
            events: self.events,
            application_data: self.application_data,
            control_statuses: self.control_statuses,
            frame_info: self.frame_info,
            requests_from_idle: self.requests_from_idle,
        }
    }
}

pub(crate) fn new_harness(config: OutstationConfig) -> OutstationHarness {
    new_harness_impl(config, None)
}

pub(crate) fn new_harness_for_broadcast(
    config: OutstationConfig,
    broadcast: BroadcastConfirmMode,
) -> OutstationHarness {
    new_harness_impl(config, Some(broadcast))
}

fn new_harness_impl(
    config: OutstationConfig,
    broadcast: Option<BroadcastConfirmMode>,
) -> OutstationHarness {
    let (sender, receiver) = event_handlers();

    let (data, application) = MockOutstationApplication::create(sender.clone());
    let control_statuses = ControlStatuses::default();
    let (requests_from_idle, information) = MockOutstationInformation::create(sender.clone());

    let (mut task, handle) = MockOutstationTask::with_transport(
        Enabled::Yes,
        LinkModes::test(),
        config,
        PhysAddr::None,
        application,
        information,
        MockControlHandler::create(sender.clone(), control_statuses.clone()),
    );

    task.set_rx_frame_info(FrameInfo::new(
        config.master_address,
        broadcast,
        FrameType::Data,
        PhysAddr::None,
    ));

    let frame_info = task.rx_frame_info();

    Harness {
        handle,
        state: spawn_session(task),
        events: receiver,
        application_data: data,
        control_statuses,
        frame_info,
        requests_from_idle,
    }
}
