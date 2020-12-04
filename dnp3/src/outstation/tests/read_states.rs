use crate::app::flags::Flags;
use crate::app::measurement::{Binary, Time};
use crate::app::types::Timestamp;
use crate::outstation::database::config::BinaryConfig;
use crate::outstation::database::{Add, EventClass, Update, UpdateOptions};
use crate::outstation::tests::harness::Event;
use crate::util::sequence::Sequence;

const EMPTY_READ: &[u8] = &[0xC0, 0x01];
const READ_CLASS_123: &[u8] = &[0xC0, 0x01, 60, 02, 0x06, 60, 03, 0x06, 60, 04, 0x06];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const BINARY_EVENT_RESPONSE: &[u8] = &[
    0xE0, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];

#[test]
fn empty_read_yields_empty_response() {
    let mut harness = super::harness::new_harness();
    harness.test_request_response(EMPTY_READ, EMPTY_RESPONSE);
    harness.check_no_events();
}

#[test]
fn can_read_and_confirm_events() {
    let mut harness = super::harness::new_harness();

    harness.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryConfig::default());
        db.update(
            0,
            &Binary::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
            UpdateOptions::default(),
        );
    });

    harness.test_request_response(READ_CLASS_123, BINARY_EVENT_RESPONSE);
    harness.check_events(&[Event::ExpectSolicitedConfirm(Sequence::new(0))]);
    harness.send(CONFIRM_SEQ_0);
    harness.check_events(&[Event::SolicitedConfirmReceived(Sequence::new(0))]);
}
