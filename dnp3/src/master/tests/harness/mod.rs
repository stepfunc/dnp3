use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Poll;

use crate::app::MaybeAsync;
use crate::decode::AppDecodeLevel;
use crate::link::header::{FrameInfo, FrameType};
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::association::AssociationConfig;
use crate::master::handler::{AssociationHandle, HeaderInfo, MasterChannel, ReadHandler};
use crate::master::session::{MasterSession, RunError};
use crate::master::{DefaultAssociationHandler, ReadType};
use crate::tokio::test::*;
use crate::transport::create_master_transport_layer;
use crate::util::phys::PhysLayer;

pub(crate) mod requests;

pub(crate) fn create_association(
    config: AssociationConfig,
) -> TestHarness<impl Future<Output = RunError>> {
    let (io, io_handle) = io::mock();

    let mut io = PhysLayer::Mock(io);

    let outstation_address = EndpointAddress::from(1024).unwrap();

    // Create the master session
    let (tx, rx) = crate::util::channel::request_channel();
    let mut runner = MasterSession::new(
        true,
        AppDecodeLevel::ObjectValues.into(),
        crate::app::Timeout::from_secs(1).unwrap(),
        MasterSession::MIN_TX_BUFFER_SIZE,
        rx,
    );
    let mut master = MasterChannel::new(tx);

    let (mut reader, mut writer) = create_master_transport_layer(
        LinkErrorMode::Close,
        EndpointAddress::from(1).unwrap(),
        MasterSession::MIN_RX_BUFFER_SIZE,
    );

    reader
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(outstation_address, None, FrameType::Data));

    let mut master_task = spawn(async move { runner.run(&mut io, &mut writer, &mut reader).await });

    // Create the association
    let handler = CountHandler::new();
    let num_requests = handler.num_requests.clone();
    let association = {
        let mut add_task = spawn(master.add_association(
            outstation_address,
            config,
            Box::new(handler),
            DefaultAssociationHandler::boxed(),
        ));
        assert_pending!(add_task.poll());
        assert_pending!(master_task.poll());
        assert_ready!(add_task.poll()).unwrap()
    };

    TestHarness {
        session: master_task,
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
    fn begin_fragment(
        &mut self,
        _read_type: ReadType,
        _header: crate::app::ResponseHeader,
    ) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }

    fn end_fragment(
        &mut self,
        _read_type: ReadType,
        _header: crate::app::ResponseHeader,
    ) -> MaybeAsync<()> {
        MaybeAsync::ready(())
    }

    fn handle_binary_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::BinaryInput, u16)>,
    ) {
    }

    fn handle_double_bit_binary_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::DoubleBitBinaryInput, u16)>,
    ) {
    }

    fn handle_binary_output_status(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::BinaryOutputStatus, u16)>,
    ) {
    }

    fn handle_counter(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::Counter, u16)>,
    ) {
    }

    fn handle_frozen_counter(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::FrozenCounter, u16)>,
    ) {
    }

    fn handle_analog_input(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::AnalogInput, u16)>,
    ) {
        self.num_requests.fetch_add(1, Ordering::SeqCst);
    }

    fn handle_analog_output_status(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::AnalogOutputStatus, u16)>,
    ) {
    }

    fn handle_octet_string<'a>(
        &mut self,
        _info: HeaderInfo,
        _iter: &'a mut dyn Iterator<Item = (crate::app::Bytes<'a>, u16)>,
    ) {
    }
}

pub(crate) struct TestHarness<F: Future<Output = RunError>> {
    pub(crate) session: Spawn<F>,
    pub(crate) master: MasterChannel,
    pub(crate) association: AssociationHandle,
    pub(crate) num_requests: Arc<AtomicU64>,
    pub(crate) io: io::Handle,
}

impl<F: Future<Output = RunError>> TestHarness<F> {
    pub(crate) fn poll(&mut self) -> Poll<RunError> {
        self.session.poll()
    }

    pub(crate) fn assert_io(&mut self) {
        assert_pending!(self.poll());
        assert!(
            self.io.all_written(),
            "Some expected bytes were not written"
        );
        assert!(self.io.all_read(), "Some bytess were not read");
    }

    pub(crate) fn num_requests(&self) -> u64 {
        self.num_requests.fetch_add(0, Ordering::Relaxed)
    }
}
