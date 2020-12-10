use crate::app::sequence::Sequence;
use crate::prelude::master::*;
use crate::tokio::test::*;
use crate::tokio::time;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::harness::create_association;
use super::harness::requests::*;

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
