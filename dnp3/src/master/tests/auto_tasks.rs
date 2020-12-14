use crate::app::header::{IIN, IIN1, IIN2};
use crate::app::sequence::Sequence;
use crate::prelude::master::*;

use super::harness::create_association;
use super::harness::requests::*;

#[test]
fn auto_integrity_scan_on_buffer_overflow() {
    let mut config = Configuration::default();
    config.auto_integrity_scan_on_buffer_overflow = true;
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        IIN::new(IIN1::new(0x00), IIN2::new(0x08)),
    );
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // Send integrity poll
    integrity_poll_request(&mut harness.io, seq);
    empty_response(&mut harness.io, seq.increment());
    harness.assert_io();
}

#[test]
fn auto_integrity_scan_on_buffer_overflow_disabled() {
    let mut config = Configuration::default();
    config.auto_integrity_scan_on_buffer_overflow = false;
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        IIN::new(IIN1::new(0x00), IIN2::new(0x08)),
    );
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // No integrity poll
    assert!(!harness.io.pending_write());
}
