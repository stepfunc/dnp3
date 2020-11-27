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

fn create_association(config: Configuration) -> TestHarness<impl Future<Output = RunError>> {
    let (mut io, io_handle) = io::mock();

    let outstation_address = EndpointAddress::from(1024).unwrap();

    // Create the master session
    let (tx, rx) = crate::tokio::sync::mpsc::channel(1);
    let mut runner = MasterSession::new(
        DecodeLogLevel::ObjectValues,
        Timeout::from_secs(1).unwrap(),
        rx,
    );
    let mut master = MasterHandle::new(tx);

    let (mut reader, mut writer) = create_master_transport_layer(EndpointAddress::from(1).unwrap());

    reader
        .get_inner()
        .set_rx_frame_info(FrameInfo::new(outstation_address, None));

    let mut master_task = spawn(async move { runner.run(&mut io, &mut writer, &mut reader).await });

    // Create the association
    let association = {
        let mut add_task =
            spawn(master.add_association(outstation_address, config, NullHandler::boxed()));
        assert_pending!(add_task.poll());
        assert_pending!(master_task.poll());
        assert_ready!(add_task.poll()).unwrap()
    };

    TestHarness {
        session: master_task,
        master,
        association,
        io: io_handle,
    }
}

struct TestHarness<F: Future<Output = RunError>> {
    session: Spawn<F>,
    master: MasterHandle,
    association: AssociationHandle,
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
}
