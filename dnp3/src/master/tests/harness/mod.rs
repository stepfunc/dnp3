use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::app::{BufferSize, Timeout};
use crate::decode::AppDecodeLevel;
use crate::link::header::{FrameInfo, FrameType};
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::association::AssociationConfig;
use crate::master::handler::{AssociationHandle, MasterChannel, ReadHandler};
use crate::master::session::{MasterSession, RunError};
use crate::master::{AssociationHandler, AssociationInformation, HeaderInfo};
use crate::transport::create_master_transport_layer;
use crate::util::phys::PhysLayer;

pub(crate) mod requests;

struct DefaultAssociationHandler;
impl AssociationHandler for DefaultAssociationHandler {}

pub(crate) async fn create_association(mut config: AssociationConfig) -> TestHarness {
    // use a 1 second timeout for all tests
    config.response_timeout = Timeout::from_secs(1).unwrap();

    let (io, io_handle) = sfio_tokio_mock_io::mock();

    let mut io = PhysLayer::Mock(io);

    let outstation_address = EndpointAddress::try_new(1024).unwrap();

    // Create the master session
    let (tx, rx) = crate::util::channel::request_channel();
    let mut runner = MasterSession::new(
        true,
        AppDecodeLevel::ObjectValues.into(),
        BufferSize::min(),
        rx,
    );
    let mut master = MasterChannel::new(tx);

    let (mut reader, mut writer) = create_master_transport_layer(
        LinkErrorMode::Close,
        EndpointAddress::try_new(1).unwrap(),
        BufferSize::min(),
    );

    reader
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(outstation_address, None, FrameType::Data));

    let master_task =
        tokio::spawn(async move { runner.run(&mut io, &mut writer, &mut reader).await });

    // Create the association
    let handler = CountHandler::new();
    let info = CountAssociationInformation::new();
    let num_requests = handler.num_requests.clone();
    let association = master
        .add_association(
            outstation_address,
            config,
            Box::new(handler),
            Box::new(DefaultAssociationHandler),
            Box::new(info),
        )
        .await
        .unwrap();

    TestHarness {
        task: master_task,
        master,
        association,
        num_requests,
        io: io_handle,
    }
}

struct CountHandler {
    num_requests: Arc<AtomicU64>,
}

impl CountHandler {
    fn new() -> Self {
        Self {
            num_requests: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ReadHandler for CountHandler {
    fn handle_analog_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::AnalogInput, u16)>,
    ) {
        self.num_requests.fetch_add(1, Ordering::SeqCst);
    }
}

struct CountAssociationInformationInner {
    last_start: Option<crate::app::Sequence>,
    num_success: u32,
    num_fail: u32,
    num_unsol: u32,
}

struct CountAssociationInformation {
    inner: Mutex<CountAssociationInformationInner>,
}

impl CountAssociationInformation {
    fn new() -> Self {
        Self {
            inner: Mutex::new(CountAssociationInformationInner {
                last_start: None,
                num_success: 0,
                num_fail: 0,
                num_unsol: 0,
            }),
        }
    }
}

impl AssociationInformation for CountAssociationInformation {
    fn task_start(
        &mut self,
        _task_type: crate::master::TaskType,
        _fc: crate::app::FunctionCode,
        seq: crate::app::Sequence,
    ) {
        let mut inner = self.inner.lock().unwrap();

        assert!(inner.last_start.is_none());

        inner.last_start = Some(seq);
    }

    fn task_success(
        &mut self,
        _task_type: crate::master::TaskType,
        _fc: crate::app::FunctionCode,
        seq: crate::app::Sequence,
    ) {
        let mut inner = self.inner.lock().unwrap();

        assert_eq!(inner.last_start, Some(seq));

        inner.num_success += 1;
        inner.last_start = None;
    }

    fn task_fail(&mut self, _task_type: crate::master::TaskType, _error: crate::master::TaskError) {
        let mut inner = self.inner.lock().unwrap();

        assert!(inner.last_start.is_some());

        inner.num_fail += 1;
        inner.last_start = None;
    }

    fn unsolicited_response(&mut self, _is_duplicate: bool, _seq: crate::app::Sequence) {
        let mut inner = self.inner.lock().unwrap();

        inner.num_unsol += 1;
    }
}

pub(crate) struct TestHarness {
    pub(crate) task: JoinHandle<RunError>,
    pub(crate) master: MasterChannel,
    pub(crate) association: AssociationHandle,
    pub(crate) num_requests: Arc<AtomicU64>,
    pub(crate) io: sfio_tokio_mock_io::Handle,
}

impl TestHarness {
    pub(crate) async fn expect_write(&mut self, expected: Vec<u8>) {
        assert_eq!(
            self.io.next_event().await,
            sfio_tokio_mock_io::Event::Write(expected)
        );
    }

    pub(crate) async fn expect_write_and_respond(&mut self, expected: Vec<u8>, response: Vec<u8>) {
        self.expect_write(expected).await;
        self.process_response(response).await;
    }

    pub(crate) async fn read_and_expect_write(&mut self, read: Vec<u8>, expected: Vec<u8>) {
        self.process_response(read).await;
        self.expect_write(expected).await;
    }

    pub(crate) async fn process_response(&mut self, data: Vec<u8>) {
        self.io.read(&data);
        assert_eq!(self.io.next_event().await, sfio_tokio_mock_io::Event::Read);
    }

    pub(crate) fn num_requests(&self) -> u64 {
        self.num_requests.fetch_add(0, Ordering::Relaxed)
    }
}
