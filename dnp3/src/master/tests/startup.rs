use crate::app::format::write::{start_request, start_response};
use crate::app::header::{Control, ResponseFunction, IIN, IIN1, IIN2};
use crate::app::sequence::Sequence;
use crate::link::header::FrameInfo;
use crate::master::session::{MasterSession, RunError};
use crate::prelude::master::*;
use crate::tokio::test::*;
use crate::tokio::time;
use crate::transport::create_master_transport_layer;
use crate::util::cursor::WriteCursor;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::task::Poll;
use std::time::Duration;

#[test]
fn master_startup_procedure() {
    let config = Configuration::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn master_startup_procedure_skips_unsolicited_if_none() {
    let mut config = Configuration::default();
    config.startup_integrity_classes = Classes::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // NO Integrity poll

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Unsolicited NULL response with RESTART IIN
    unsol_null(&mut harness.io, seq, true);
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // NO Integrity poll

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn master_startup_procedure_skips_integrity_poll_if_none() {
    let mut config = Configuration::default();
    config.disable_unsol_classes = EventClasses::none();
    config.enable_unsol_classes = EventClasses::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Only integrity poll is needed
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn clear_restart_iin_is_higher_priority() {
    let config = Configuration::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    harness.assert_io();

    // Never respond to it, send unsolicited NULL response with RESTART IIN
    unsol_null(&mut harness.io, seq, true);
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // Respond to the DISABLE_UNSOLICITED
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Now clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Proceed with the rest of the startup sequence

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn outstation_restart_procedure() {
    let config = Configuration::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited NULL response with DEVICE_RESTART IIN
    unsol_null(&mut harness.io, seq, true);
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn ignore_unsolicited_response_with_data_before_first_integrity_poll() {
    let config = Configuration::default();
    let mut seq = Sequence::default();
    let mut unsol_seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited NULL response with DEVICE_RESTART IIN
    unsol_null(&mut harness.io, unsol_seq, true);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.assert_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll (never respond to)
    integrity_poll_request(&mut harness.io, seq.increment());
    harness.assert_io();

    // Send unsolicited with data
    unsol_with_data(&mut harness.io, unsol_seq, 42, true);
    harness.assert_io();

    // DO NOT CONFIRM AND IGNORE PAYLOAD
    assert_eq!(harness.num_requests.fetch_add(0, Ordering::Relaxed), 0);

    // Instead, wait for the integrity poll to timeout then
    // restart the outstation startup procedure
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn ignore_duplicate_unsolicited_response() {
    let config = Configuration::default();
    let mut seq = Sequence::default();
    let mut unsol_seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Send unsolicited with data
    unsol_with_data(&mut harness.io, unsol_seq, 42, false);
    unsol_confirm(&mut harness.io, unsol_seq);
    harness.assert_io();
    assert_eq!(harness.num_requests(), 1);

    // Send exact same unsolicited response
    unsol_with_data(&mut harness.io, unsol_seq, 42, false);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.assert_io();
    assert_eq!(harness.num_requests(), 1);

    // Send different data
    unsol_with_data(&mut harness.io, unsol_seq, 43, false);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.assert_io();
    assert_eq!(harness.num_requests(), 2);

    // Send different sequence number
    unsol_with_data(&mut harness.io, unsol_seq, 43, false);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.assert_io();
    assert_eq!(harness.num_requests(), 3);
}

#[test]
fn master_startup_retry_procedure() {
    let mut config = Configuration::default();
    config.auto_tasks_retry_strategy =
        RetryStrategy::new(Duration::from_secs(1), Duration::from_secs(3));
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // First disable unsolicited
    disable_unsol_request(&mut harness.io, seq.increment());
    harness.assert_io();

    // Wait for the timeout
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    // Wait before retransmitting
    time::advance(Duration::from_millis(999));
    assert_pending!(harness.poll());
    assert!(!harness.io.pending_write());

    // First retransmit
    time::advance(Duration::from_millis(1));
    disable_unsol_request(&mut harness.io, seq.increment());
    harness.assert_io();

    // Wait for the timeout
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    // Wait before retransmitting
    time::advance(Duration::from_millis(1999));
    assert_pending!(harness.poll());
    assert!(!harness.io.pending_write());

    // Second retransmit
    time::advance(Duration::from_millis(1));
    disable_unsol_request(&mut harness.io, seq.increment());
    harness.assert_io();

    // Wait for the timeout
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    // Wait before retransmitting (reaching max delay)
    time::advance(Duration::from_millis(2999));
    assert_pending!(harness.poll());
    assert!(!harness.io.pending_write());

    // Third retransmit
    time::advance(Duration::from_millis(1));
    disable_unsol_request(&mut harness.io, seq);
    harness.assert_io();

    // Actually answer it and complete the startup procedure
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

fn disable_unsol_request(io: &mut io::Handle, seq: Sequence) {
    // DISABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request = start_request(
        Control::request(seq),
        FunctionCode::DisableUnsolicited,
        &mut cursor,
    )
    .unwrap();

    request
        .write_all_objects_header(Variation::Group60Var2)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var3)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var4)
        .unwrap();

    io.write(cursor.written());
}

fn integrity_poll_request(io: &mut io::Handle, seq: Sequence) {
    // Integrity poll
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(Control::request(seq), FunctionCode::Read, &mut cursor).unwrap();

    request.write_class1230().unwrap();

    io.write(cursor.written());
}

fn enable_unsol_request(io: &mut io::Handle, seq: Sequence) {
    // ENABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request = start_request(
        Control::request(seq),
        FunctionCode::EnableUnsolicited,
        &mut cursor,
    )
    .unwrap();

    request
        .write_all_objects_header(Variation::Group60Var2)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var3)
        .unwrap();
    request
        .write_all_objects_header(Variation::Group60Var4)
        .unwrap();

    io.write(cursor.written());
}

fn clear_restart_iin(io: &mut io::Handle, seq: Sequence) {
    // ENABLE_UNSOLICITED request
    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut request =
        start_request(Control::request(seq), FunctionCode::Write, &mut cursor).unwrap();

    request.write_clear_restart().unwrap();

    io.write(cursor.written());
}

fn empty_response(io: &mut io::Handle, seq: Sequence) {
    let mut buffer = [0; 4];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_response(
        Control::response(seq, true, true, false),
        ResponseFunction::Response,
        IIN::default(),
        &mut cursor,
    )
    .unwrap();

    io.read(cursor.written());
}

// Unsolicited stuff

fn unsol_null(io: &mut io::Handle, seq: Sequence, restart_iin: bool) {
    let iin = if restart_iin {
        IIN::new(IIN1::new(0x80), IIN2::new(0x00))
    } else {
        IIN::default()
    };

    let mut buffer = [0; 4];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_response(
        Control::unsolicited_response(seq),
        ResponseFunction::UnsolicitedResponse,
        iin,
        &mut cursor,
    )
    .unwrap();

    io.read(cursor.written());
}

fn unsol_confirm(io: &mut io::Handle, seq: Sequence) {
    let mut buffer = [0; 2];
    let mut cursor = WriteCursor::new(&mut buffer);
    start_request(
        Control::unsolicited(seq),
        FunctionCode::Confirm,
        &mut cursor,
    )
    .unwrap();

    io.write(cursor.written());
}

fn unsol_with_data(io: &mut io::Handle, seq: Sequence, data: i16, restart_iin: bool) {
    let iin = if restart_iin {
        IIN::new(IIN1::new(0x80), IIN2::new(0x00))
    } else {
        IIN::default()
    };

    let mut buffer = [0; 20];
    let mut cursor = WriteCursor::new(&mut buffer);
    let mut response = start_response(
        Control::unsolicited_response(seq),
        ResponseFunction::UnsolicitedResponse,
        iin,
        &mut cursor,
    )
    .unwrap();

    response
        .write_prefixed_items(
            [(
                Group32Var2 {
                    value: data,
                    flags: 0x00,
                },
                0u8,
            )]
            .iter(),
        )
        .unwrap();

    io.read(cursor.written());
}

// Whole procedure
fn startup_procedure<F: Future<Output = RunError>>(
    harness: &mut TestHarness<F>,
    seq: &mut Sequence,
) {
    // Disable unsolicited
    disable_unsol_request(&mut harness.io, *seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, *seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, *seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

fn create_association(config: Configuration) -> TestHarness<impl Future<Output = RunError>> {
    let (mut io, io_handle) = io::mock();

    let outstation_address = EndpointAddress::from(1024).unwrap();

    // Create the master session
    let (tx, rx) = crate::tokio::sync::mpsc::channel(1);
    let mut runner = MasterSession::new(
        DecodeLogLevel::ObjectValues,
        Timeout::from_secs(1).unwrap(),
        MasterSession::MIN_TX_BUFFER_SIZE,
        rx,
    );
    let mut master = MasterHandle::new(tx);

    let (mut reader, mut writer) = create_master_transport_layer(
        EndpointAddress::from(1).unwrap(),
        MasterSession::MIN_RX_BUFFER_SIZE,
    );

    reader
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(outstation_address, None));

    let mut master_task = spawn(async move { runner.run(&mut io, &mut writer, &mut reader).await });

    // Create the association
    let handler = CountHandler::new();
    let num_requests = handler.num_requests.clone();
    let association = {
        let mut add_task =
            spawn(master.add_association(outstation_address, config, Box::new(handler)));
        assert_pending!(add_task.poll());
        assert_pending!(master_task.poll());
        assert_ready!(add_task.poll()).unwrap()
    };

    TestHarness {
        session: master_task,
        _master: master,
        _association: association,
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

impl AssociationHandler for CountHandler {
    fn get_integrity_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }

    fn get_unsolicited_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }

    fn get_default_poll_handler(&mut self) -> &mut dyn ReadHandler {
        self
    }
}

impl ReadHandler for CountHandler {
    fn begin_fragment(&mut self, _header: crate::app::header::ResponseHeader) {}

    fn end_fragment(&mut self, _header: crate::app::header::ResponseHeader) {}

    fn handle_binary(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::Binary, u16)>,
    ) {
    }

    fn handle_double_bit_binary(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::DoubleBitBinary, u16)>,
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

    fn handle_analog(
        &mut self,
        _info: HeaderInfo,
        _iter: &mut dyn Iterator<Item = (crate::app::measurement::Analog, u16)>,
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
        _iter: &'a mut dyn Iterator<Item = (crate::app::parse::bytes::Bytes<'a>, u16)>,
    ) {
    }
}

struct TestHarness<F: Future<Output = RunError>> {
    session: Spawn<F>,
    _master: MasterHandle,
    _association: AssociationHandle,
    num_requests: Arc<AtomicU64>,
    io: io::Handle,
}

impl<F: Future<Output = RunError>> TestHarness<F> {
    fn poll(&mut self) -> Poll<RunError> {
        self.session.poll()
    }

    fn assert_io(&mut self) {
        assert_pending!(self.poll());
        assert!(self.io.all_done());
    }

    fn num_requests(&self) -> u64 {
        self.num_requests.fetch_add(0, Ordering::Relaxed)
    }
}
