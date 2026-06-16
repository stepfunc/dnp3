use crate::app::parse::options::ParseOptions;
use crate::decode::AppDecodeLevel;
use crate::link::header::{BroadcastConfirmMode, FrameInfo, FrameType};
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::EventBufferConfig;
use crate::outstation::task::OutstationTask;
use crate::outstation::tests::harness::{
    event_handlers, ApplicationData, Event, EventReceiver, MockControlHandler,
    MockOutstationApplication, MockOutstationInformation,
};
use crate::outstation::OutstationHandle;
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, RunError};
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

type Session = JoinHandle<(OutstationTask, RunError)>;

pub(crate) struct Harness<S> {
    pub(crate) handle: OutstationHandle,
    state: S,
    events: EventReceiver,
    pub(crate) application_data: Arc<Mutex<ApplicationData>>,
}

/// connected state: a live mock IO handle and the running session
pub(crate) struct Connected {
    io: sfio_tokio_mock_io::Handle,
    session: Session,
}

/// disconnected state: the task (and its `DatabaseHandle`) parked between connections
pub(crate) struct Disconnected {
    task: OutstationTask,
}

async fn run_session(mut task: OutstationTask, mut io: PhysLayer) -> (OutstationTask, RunError) {
    let err = task.run(&mut io).await;
    (task, err)
}

fn spawn_session(task: OutstationTask) -> Connected {
    let (io, io_handle) = sfio_tokio_mock_io::mock();
    let phys = PhysLayer::Mock(io);
    Connected {
        io: io_handle,
        session: tokio::spawn(run_session(task, phys)),
    }
}

// methods available in any connection state
impl<S> Harness<S> {
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

    pub(crate) fn check_no_events(&mut self) {
        if let Some(x) = self.events.poll() {
            panic!("expected no events, but next event is: {x:?}")
        }
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
        }
    }
}

pub(crate) fn new_harness(config: OutstationConfig) -> OutstationHarness {
    new_harness_impl(config, None, None)
}

pub(crate) fn new_harness_with_master_addr(
    config: OutstationConfig,
    master_address: EndpointAddress,
) -> OutstationHarness {
    new_harness_impl(config, None, Some(master_address))
}

pub(crate) fn new_harness_for_broadcast(
    config: OutstationConfig,
    broadcast: BroadcastConfirmMode,
) -> OutstationHarness {
    new_harness_impl(config, Some(broadcast), None)
}

fn new_harness_impl(
    config: OutstationConfig,
    broadcast: Option<BroadcastConfirmMode>,
    master_address: Option<EndpointAddress>,
) -> OutstationHarness {
    let (sender, receiver) = event_handlers();

    let (data, application) = MockOutstationApplication::create(sender.clone());

    let (mut task, handle) = OutstationTask::create(
        Enabled::Yes,
        LinkModes::test(),
        ParseOptions::get_static(),
        config,
        PhysAddr::None,
        application,
        MockOutstationInformation::create(sender.clone()),
        MockControlHandler::create(sender.clone()),
    );

    let master_address = master_address.unwrap_or(config.master_address);

    task.get_reader()
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(
            master_address,
            broadcast,
            FrameType::Data,
            PhysAddr::None,
        ));

    Harness {
        handle,
        state: spawn_session(task),
        events: receiver,
        application_data: data,
    }
}
