use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::app::measurement::AnalogInputDeadBand;
use crate::app::{BufferSize, Timeout};
use crate::app::{MaybeAsync, ResponseHeader};
use crate::decode::AppDecodeLevel;
use crate::link::header::{FrameInfo, FrameType};
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::association::{AssociationConfig, AssociationMap};
use crate::master::task::MasterTask;
use crate::master::{AssociationHandle, MasterChannel, ReadHandler, ReadType};
use crate::master::{
    AssociationHandler, AssociationInformation, HeaderInfo, MasterChannelConfig, MasterChannelType,
};
use crate::transport::mock::reader::RxFrameInfo;
use crate::transport::mock::MockTransport;
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, RunError};

pub(crate) mod requests;

struct DefaultAssociationHandler;
impl AssociationHandler for DefaultAssociationHandler {}

pub(crate) async fn create_association(config: AssociationConfig) -> TestHarness {
    create_association_with_handler(config, Box::new(DefaultAssociationHandler)).await
}

pub(crate) async fn create_association_with_handler(
    mut config: AssociationConfig,
    assoc_handler: Box<dyn AssociationHandler>,
) -> TestHarness {
    // use a 1-second timeout for all tests
    config.response_timeout = Timeout::from_secs(1).unwrap();

    let (io, io_handle) = sfio_tokio_mock_io::mock();

    let mut io = PhysLayer::mock(io);

    let outstation_address = EndpointAddress::try_new(1024).unwrap();

    let task_config = MasterChannelConfig {
        master_address: EndpointAddress::try_new(1).unwrap(),
        decode_level: AppDecodeLevel::ObjectValues.into(),
        tx_buffer_size: BufferSize::min(),
        rx_buffer_size: BufferSize::min(),
    };

    // Create the master session
    let (tx, rx) = crate::util::channel::request_channel();
    let mut task = MasterTask::<MockTransport>::with_transport(
        Enabled::Yes,
        AssociationMap::new(),
        LinkModes::stream(LinkErrorMode::Discard),
        task_config,
        rx,
    );

    let mut master = MasterChannel::new(tx, MasterChannelType::Stream);

    task.set_rx_frame_info(FrameInfo::new(
        outstation_address,
        None,
        FrameType::Data,
        PhysAddr::None,
    ));

    let rx_info = task.rx_frame_info();
    let master_task = tokio::spawn(async move { task.run(&mut io).await });

    // Create the association
    let handler = CountHandler::new();
    let (info, assoc_events) = AssociationInformationEventHandler::pair();
    let num_requests = handler.num_requests.clone();
    let read_events = handler.events();

    let association = master
        .add_association(
            outstation_address,
            config,
            Box::new(handler),
            assoc_handler,
            Box::new(info),
        )
        .await
        .unwrap();

    TestHarness {
        task: master_task,
        master,
        association,
        num_requests,
        assoc_events,
        read_events,
        io: io_handle,
        rx_info,
    }
}

/// A ReadHandler callback, in the order the handler received it
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum ReadEvent {
    BeginFragment(ReadType),
    BinaryInput,
    DoubleBitBinaryInput,
    AnalogInput,
    /// one deadband of a header, and its index
    AnalogInputDeadBand(AnalogInputDeadBand, u16),
    EndFragment(ReadType),
}

pub(crate) struct ReadEventQueue(Arc<Mutex<Vec<ReadEvent>>>);

impl ReadEventQueue {
    pub(crate) fn pop(&mut self) -> Vec<ReadEvent> {
        let mut guard = self.0.lock().unwrap();
        guard.drain(..).collect()
    }
}

pub(crate) struct CountHandler {
    num_requests: Arc<AtomicU64>,
    events: Arc<Mutex<Vec<ReadEvent>>>,
}

impl CountHandler {
    pub(crate) fn new() -> Self {
        Self {
            num_requests: Arc::new(AtomicU64::new(0)),
            events: Default::default(),
        }
    }

    pub(crate) fn events(&self) -> ReadEventQueue {
        ReadEventQueue(self.events.clone())
    }

    fn push(&mut self, event: ReadEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl ReadHandler for CountHandler {
    fn begin_fragment(&mut self, read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        self.push(ReadEvent::BeginFragment(read_type));
        MaybeAsync::ready(())
    }

    fn end_fragment(&mut self, read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        self.push(ReadEvent::EndFragment(read_type));
        MaybeAsync::ready(())
    }

    fn handle_binary_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::BinaryInput, u16)>,
    ) {
        self.push(ReadEvent::BinaryInput);
    }

    fn handle_double_bit_binary_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::DoubleBitBinaryInput, u16)>,
    ) {
        self.push(ReadEvent::DoubleBitBinaryInput);
    }

    fn handle_analog_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::AnalogInput, u16)>,
    ) {
        self.num_requests.fetch_add(1, Ordering::SeqCst);
        self.push(ReadEvent::AnalogInput);
    }

    fn handle_analog_input_dead_band(
        &mut self,
        _info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
        for (value, index) in iter {
            self.push(ReadEvent::AnalogInputDeadBand(value, index));
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum AssocInfoEvent {
    TaskStart(
        crate::master::TaskType,
        crate::app::FunctionCode,
        crate::app::Sequence,
    ),
    TaskSuccess(
        crate::master::TaskType,
        crate::app::FunctionCode,
        crate::app::Sequence,
    ),
    TaskFailure(crate::master::TaskType, crate::master::TaskError),
    Unsolicited(bool, crate::app::Sequence),
}

pub(crate) struct AssocInfoEventQueue(Arc<Mutex<Vec<AssocInfoEvent>>>);

impl AssocInfoEventQueue {
    pub(crate) fn pop(&mut self) -> Vec<AssocInfoEvent> {
        let mut guard = self.0.lock().unwrap();
        guard.drain(..).collect()
    }
}

pub(crate) struct AssociationInformationEventHandler(Arc<Mutex<Vec<AssocInfoEvent>>>);

impl AssociationInformationEventHandler {
    pub(crate) fn pair() -> (Self, AssocInfoEventQueue) {
        let events: Arc<Mutex<Vec<AssocInfoEvent>>> = Default::default();
        (Self(events.clone()), AssocInfoEventQueue(events))
    }

    fn push(&mut self, event: AssocInfoEvent) {
        self.0.lock().unwrap().push(event);
    }
}

impl AssociationInformation for AssociationInformationEventHandler {
    fn task_start(
        &mut self,
        task_type: crate::master::TaskType,
        fc: crate::app::FunctionCode,
        seq: crate::app::Sequence,
    ) {
        self.push(AssocInfoEvent::TaskStart(task_type, fc, seq));
    }

    fn task_success(
        &mut self,
        task_type: crate::master::TaskType,
        fc: crate::app::FunctionCode,
        seq: crate::app::Sequence,
    ) {
        self.push(AssocInfoEvent::TaskSuccess(task_type, fc, seq));
    }

    fn task_fail(&mut self, task_type: crate::master::TaskType, error: crate::master::TaskError) {
        self.push(AssocInfoEvent::TaskFailure(task_type, error));
    }

    fn unsolicited_response(&mut self, is_duplicate: bool, seq: crate::app::Sequence) {
        self.push(AssocInfoEvent::Unsolicited(is_duplicate, seq));
    }
}

pub(crate) struct TestHarness {
    pub(crate) task: JoinHandle<RunError>,
    pub(crate) master: MasterChannel,
    pub(crate) association: AssociationHandle,
    pub(crate) num_requests: Arc<AtomicU64>,
    pub(crate) assoc_events: AssocInfoEventQueue,
    pub(crate) read_events: ReadEventQueue,
    pub(crate) io: sfio_tokio_mock_io::Handle,
    rx_info: RxFrameInfo,
}

impl TestHarness {
    /// Set the link address that the fragments read after this appear to come from
    pub(crate) fn set_source(&self, source: EndpointAddress) {
        self.rx_info.set(FrameInfo::new(
            source,
            None,
            FrameType::Data,
            PhysAddr::None,
        ));
    }

    pub(crate) async fn expect_write(&mut self, expected: Vec<u8>) {
        assert_eq!(
            self.io.next_event().await,
            sfio_tokio_mock_io::Event::Write(expected)
        );
    }

    pub(crate) fn assert_no_events(&mut self) {
        assert_eq!(self.io.pop_event(), None);
    }

    pub(crate) async fn pop_write(&mut self) -> Vec<u8> {
        match self.io.next_event().await {
            sfio_tokio_mock_io::Event::Write(data) => data,
            x => panic!("expected a write, but got: {x:?}"),
        }
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

    /// Check the time elapsed on the paused clock, allowing for tokio rounding each timer up to the
    /// next millisecond
    pub(crate) fn assert_elapsed(since: tokio::time::Instant, expected: std::time::Duration) {
        let elapsed = since.elapsed();
        assert!(
            elapsed >= expected
                && elapsed.saturating_sub(expected) < std::time::Duration::from_millis(10),
            "expected {expected:?}, elapsed {elapsed:?}"
        );
    }

    pub(crate) fn num_requests(&self) -> u64 {
        self.num_requests.fetch_add(0, Ordering::Relaxed)
    }
}
