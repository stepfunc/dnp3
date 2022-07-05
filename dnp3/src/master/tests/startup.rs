use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::app::format::write::start_request;
use crate::app::variations::Variation;
use crate::app::FunctionCode;
use crate::app::RetryStrategy;
use crate::app::Sequence;
use crate::app::{ControlField, Iin, Iin1, Iin2};
use crate::master::association::AssociationConfig;
use crate::master::request::{Classes, EventClasses};
use crate::master::ReadRequest;
use crate::util::cursor::WriteCursor;

use tokio::time;

use super::harness::create_association;
use super::harness::requests::*;

#[test]
fn master_startup_procedure() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn master_startup_procedure_skips_unsolicited_if_none() {
    let mut config = AssociationConfig::default();
    config.startup_integrity_classes = Classes::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // NO Integrity poll

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Unsolicited NULL response with RESTART IIN
    unsol_null(&mut harness.io, seq, true);
    unsol_confirm(&mut harness.io, seq);
    harness.flush_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // NO Integrity poll

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn master_startup_procedure_skips_integrity_poll_if_none() {
    let mut config = AssociationConfig::default();
    config.disable_unsol_classes = EventClasses::none();
    config.enable_unsol_classes = EventClasses::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Only integrity poll is needed
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn clear_restart_iin_is_higher_priority() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // Disable unsolicited
    disable_unsol_request(&mut harness.io, seq);
    harness.flush_io();

    // Never respond to it, send unsolicited NULL response with RESTART IIN
    unsol_null(&mut harness.io, seq, true);
    unsol_confirm(&mut harness.io, seq);
    harness.flush_io();

    // Respond to the DISABLE_UNSOLICITED
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Now clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Proceed with the rest of the startup sequence

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn outstation_restart_procedure() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited NULL response with DEVICE_RESTART IIN
    unsol_null(&mut harness.io, seq, true);
    unsol_confirm(&mut harness.io, seq);
    harness.flush_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn detect_restart_in_read_response() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Send Class 0 read request
    {
        let mut read_task = spawn(
            harness
                .association
                .read(ReadRequest::ClassScan(Classes::class0())),
        );
        assert_pending!(read_task.poll());
    }
    harness.flush_io();

    {
        // Read class 0 data
        let mut buffer = [0; 20];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut request =
            start_request(ControlField::request(seq), FunctionCode::Read, &mut cursor).unwrap();

        request
            .write_all_objects_header(Variation::Group60Var1)
            .unwrap();

        harness.io.write(cursor.written());
    }

    // Response with DEVICE_RESTART IIN bit set
    empty_response_custom_iin(
        &mut harness.io,
        seq.increment(),
        Iin::new(Iin1::new(0x80), Iin2::new(0x00)),
    );
    harness.flush_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn ignore_unsolicited_response_with_data_before_first_integrity_poll() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut unsol_seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited NULL response with DEVICE_RESTART IIN
    unsol_null(&mut harness.io, unsol_seq, true);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.flush_io();

    // Clear the restart flag
    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Integrity poll (never respond to)
    integrity_poll_request(&mut harness.io, seq.increment());
    harness.flush_io();

    // Send unsolicited with data
    unsol_with_data(&mut harness.io, unsol_seq, 42, true);
    harness.flush_io();

    // DO NOT CONFIRM AND IGNORE PAYLOAD
    assert_eq!(harness.num_requests.fetch_add(0, Ordering::Relaxed), 0);

    // Instead, wait for the integrity poll to timeout then
    // restart the outstation startup procedure
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    clear_restart_iin(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Integrity poll
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}

#[test]
fn ignore_duplicate_unsolicited_response() {
    let config = AssociationConfig::default();
    let mut seq = Sequence::default();
    let mut unsol_seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Send unsolicited with data
    unsol_with_data(&mut harness.io, unsol_seq, 42, false);
    unsol_confirm(&mut harness.io, unsol_seq);
    harness.flush_io();
    assert_eq!(harness.num_requests(), 1);

    // Send exact same unsolicited response
    unsol_with_data(&mut harness.io, unsol_seq, 42, false);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.flush_io();
    assert_eq!(harness.num_requests(), 1);

    // Send different data
    unsol_with_data(&mut harness.io, unsol_seq, 43, false);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.flush_io();
    assert_eq!(harness.num_requests(), 2);

    // Send different sequence number
    unsol_with_data(&mut harness.io, unsol_seq, 43, false);
    unsol_confirm(&mut harness.io, unsol_seq.increment());
    harness.flush_io();
    assert_eq!(harness.num_requests(), 3);
}

#[test]
fn master_startup_retry_procedure() {
    let mut config = AssociationConfig::default();
    config.auto_tasks_retry_strategy =
        RetryStrategy::new(Duration::from_secs(1), Duration::from_secs(3));
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    // First disable unsolicited
    disable_unsol_request(&mut harness.io, seq.increment());
    harness.flush_io();

    // Wait for the timeout
    time::advance(Duration::from_secs(1));
    assert_pending!(harness.poll());

    // Wait before retransmitting
    time::advance(Duration::from_millis(999));
    harness.flush_io();

    // First retransmit
    time::advance(Duration::from_millis(1));
    disable_unsol_request(&mut harness.io, seq.increment());
    harness.flush_io();

    // Wait for the timeout
    time::advance(Duration::from_secs(1));
    harness.flush_io();

    // Wait before retransmitting
    time::advance(Duration::from_millis(1999));
    harness.flush_io();

    // Second retransmit
    time::advance(Duration::from_millis(1));
    disable_unsol_request(&mut harness.io, seq.increment());
    harness.flush_io();

    // Wait for the timeout
    time::advance(Duration::from_secs(1));
    harness.flush_io();

    // Wait before retransmitting (reaching max delay)
    time::advance(Duration::from_millis(2999));
    harness.flush_io();

    // Third retransmit
    time::advance(Duration::from_millis(1));
    disable_unsol_request(&mut harness.io, seq);
    harness.flush_io();

    // Actually answer it and complete the startup procedure
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();

    // Enable unsolicited
    enable_unsol_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.flush_io();
}
