use crate::app::format::write::start_request;
use crate::app::Sequence;
use crate::app::{Control, Iin, Iin1, Iin2};
use crate::util::cursor::WriteCursor;

use super::harness::create_association;
use super::harness::requests::*;
use crate::app::variations::Variation;
use crate::app::FunctionCode;
use crate::master::association::AssociationConfig;
use crate::master::request::EventClasses;

#[test]
fn auto_integrity_scan_on_buffer_overflow() {
    let mut config = AssociationConfig::default();
    config.auto_integrity_scan_on_buffer_overflow = true;
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        Iin::new(Iin1::new(0x00), Iin2::new(0x08)),
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
    let mut config = AssociationConfig::default();
    config.auto_integrity_scan_on_buffer_overflow = false;
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        Iin::new(Iin1::new(0x00), Iin2::new(0x08)),
    );
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // No integrity poll
    assert!(!harness.io.pending_write());
}

#[test]
fn auto_event_class_scan() {
    let mut config = AssociationConfig::default();
    config.event_scan_on_events_available = EventClasses::all();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        Iin::new(Iin1::new(0x02), Iin2::new(0x00)), // Class 1 events
    );
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    {
        // Read class 1 events
        let mut buffer = [0; 20];
        let mut cursor = WriteCursor::new(&mut buffer);
        let mut request =
            start_request(Control::request(seq), FunctionCode::Read, &mut cursor).unwrap();

        request
            .write_all_objects_header(Variation::Group60Var2)
            .unwrap();

        harness.io.write(cursor.written());
    }
    empty_response(&mut harness.io, seq);
    harness.assert_io();

    // No integrity poll
    assert!(!harness.io.pending_write());
}

#[test]
fn auto_event_class_ignore_one_class_scan() {
    let mut config = AssociationConfig::default();
    config.event_scan_on_events_available = EventClasses::new(false, true, true);
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        Iin::new(Iin1::new(0x02), Iin2::new(0x00)), // Class 1 events
    );
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // No reads
    assert!(!harness.io.pending_write());
}

#[test]
fn auto_event_class_scan_disabled() {
    let mut config = AssociationConfig::default();
    config.event_scan_on_events_available = EventClasses::none();
    let mut seq = Sequence::default();
    let mut harness = create_association(config);

    startup_procedure(&mut harness, &mut seq);

    // Unsolicited with IIN2.3 EVENT_BUFFER_OVERFLOW set
    unsol_null_custom_iin(
        &mut harness.io,
        seq,
        Iin::new(Iin1::new(0x02), Iin2::new(0x00)), // Class 1 events
    );
    unsol_confirm(&mut harness.io, seq);
    harness.assert_io();

    // No reads
    assert!(!harness.io.pending_write());
}
