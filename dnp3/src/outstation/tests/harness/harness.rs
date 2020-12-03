use crate::link::error::LinkError;
use crate::link::header::{BroadcastConfirmMode, FrameInfo};
use crate::outstation::database::{DatabaseConfig, DatabaseHandle};
use crate::outstation::task::{OutstationConfig, OutstationTask};
use crate::outstation::tests::harness::{
    Event, EventHandle, MockControlHandler, MockOutstationInformation,
};
use crate::outstation::traits::DefaultOutstationApplication;
use crate::tokio::test::*;

pub(crate) struct OutstationTestHarness<T>
where
    T: std::future::Future<Output = Result<(), LinkError>>,
{
    database: DatabaseHandle,
    io: io::Handle,
    task: Spawn<T>,
    events: EventHandle,
}

impl<T> OutstationTestHarness<T>
where
    T: std::future::Future<Output = Result<(), LinkError>>,
{
    pub(crate) fn test_request_response(&mut self, request: &[u8], response: &[u8]) {
        self.io.read(request);
        assert_pending!(self.task.poll());
        assert!(self.io.pending_write());
        self.io.write(response);
        assert_pending!(self.task.poll());
        assert!(self.io.all_done());
    }

    pub(crate) fn test_request_no_response(&mut self, request: &[u8]) {
        self.io.read(request);
        assert_pending!(self.task.poll());
        assert!(!self.io.pending_write());
        assert!(self.io.all_done());
    }

    pub(crate) fn check_events(&mut self, events: &[Event]) {
        for event in events {
            assert_eq!(Some(*event), self.events.pop());
        }
        assert_eq!(self.events.pop(), None);
    }

    pub(crate) fn check_no_events(&mut self) {
        self.check_events(&[]);
    }
}

pub(crate) fn new_harness(
    config: OutstationConfig,
) -> OutstationTestHarness<impl std::future::Future<Output = Result<(), LinkError>>> {
    new_harness_with_broadcast(config, None)
}

pub(crate) fn new_harness_with_broadcast(
    config: OutstationConfig,
    broadcast: Option<BroadcastConfirmMode>,
) -> OutstationTestHarness<impl std::future::Future<Output = Result<(), LinkError>>> {
    let events = EventHandle::new();

    let (task, database) = OutstationTask::create(
        config,
        DatabaseConfig::default(),
        DefaultOutstationApplication::create(),
        MockOutstationInformation::new(events.clone()),
        MockControlHandler::new(events.clone()),
    );

    let mut task = Box::new(task);

    task.get_reader()
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(config.master_address, broadcast));

    let (mut io, io_handle) = io::mock();

    OutstationTestHarness {
        database,
        io: io_handle,
        task: spawn(async move { task.run(&mut io).await }),
        events,
    }
}
