use std::sync::{Arc, Mutex};

use crate::decode::AppDecodeLevel;
use crate::link::header::{BroadcastConfirmMode, FrameInfo, FrameType};
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::EventBufferConfig;
use crate::outstation::session::RunError;
use crate::outstation::task::OutstationTask;
use crate::outstation::tests::harness::{
    ApplicationData, Event, EventHandle, MockControlHandler, MockOutstationApplication,
    MockOutstationInformation,
};
use crate::outstation::OutstationHandle;
use crate::tokio::test::*;
use crate::util::phys::PhysLayer;

pub(crate) fn get_default_config() -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.features.unsolicited = Feature::Disabled;
    config
}

pub(crate) fn get_default_unsolicited_config() -> OutstationConfig {
    let mut config = OutstationConfig::new(
        EndpointAddress::from(10).unwrap(),
        EndpointAddress::from(1).unwrap(),
    );

    config.decode_level = AppDecodeLevel::ObjectValues.into();

    config
}

pub(crate) struct OutstationTestHarness<T>
where
    T: std::future::Future<Output = RunError>,
{
    pub(crate) handle: OutstationHandle,
    io: io::Handle,
    task: Spawn<T>,
    events: EventHandle,
    pub(crate) application_data: Arc<Mutex<ApplicationData>>,
}

impl<T> OutstationTestHarness<T>
where
    T: std::future::Future<Output = RunError>,
{
    pub(crate) fn poll_pending(&mut self) {
        assert_pending!(self.task.poll());
    }

    pub(crate) fn test_request_response(&mut self, request: &[u8], response: &[u8]) {
        self.io.read(request);
        self.poll_pending();
        assert!(self.io.pending_write());
        self.io.write(response);
        assert_pending!(self.task.poll());
        assert!(self.io.all_read());
        assert!(self.io.all_written());
    }

    pub(crate) fn expect_response(&mut self, response: &[u8]) {
        self.io.write(response);
        self.poll_pending();
        assert!(self.io.all_written());
    }

    pub(crate) fn send(&mut self, request: &[u8]) {
        self.io.read(request);
        self.poll_pending();
    }

    pub(crate) fn test_request_no_response(&mut self, request: &[u8]) {
        self.io.read(request);
        self.poll_pending();
        assert!(!self.io.pending_write());
        self.check_all_io_consumed();
    }

    pub(crate) fn check_all_io_consumed(&mut self) {
        assert!(self.io.all_read());
        assert!(self.io.all_written());
    }

    pub(crate) fn check_events(&mut self, events: &[Event]) {
        for event in events {
            match self.events.pop() {
                None => {
                    panic!("Expected {:?} but there are no more events", event);
                }
                Some(x) => {
                    if *event != x {
                        panic!("Expected {:?} but next event is {:?}", event, x);
                    }
                }
            }
        }
        self.check_no_events();
    }

    pub(crate) fn check_no_events(&mut self) {
        if let Some(x) = self.events.pop() {
            panic!("expected no events, but next event is: {:?}", x)
        }
    }
}

pub(crate) fn new_harness(
    config: OutstationConfig,
) -> OutstationTestHarness<impl std::future::Future<Output = RunError>> {
    new_harness_impl(config, None)
}

pub(crate) fn new_harness_for_broadcast(
    config: OutstationConfig,
    broadcast: BroadcastConfirmMode,
) -> OutstationTestHarness<impl std::future::Future<Output = RunError>> {
    new_harness_impl(config, Some(broadcast))
}

fn new_harness_impl(
    config: OutstationConfig,
    broadcast: Option<BroadcastConfirmMode>,
) -> OutstationTestHarness<impl std::future::Future<Output = RunError>> {
    let events = EventHandle::new();

    let (data, application) = MockOutstationApplication::new(events.clone());

    let (task, handle) = OutstationTask::create(
        LinkErrorMode::Close,
        config,
        EventBufferConfig::all_types(5),
        application,
        MockOutstationInformation::new(events.clone()),
        MockControlHandler::new(events.clone()),
    );

    let mut task = Box::new(task);

    task.get_reader()
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(
            config.master_address,
            broadcast,
            FrameType::Data,
        ));

    let (io, io_handle) = io::mock();

    let mut io = PhysLayer::Mock(io);

    OutstationTestHarness {
        handle,
        io: io_handle,
        task: spawn(async move { task.run(&mut io).await }),
        events,
        application_data: data,
    }
}
