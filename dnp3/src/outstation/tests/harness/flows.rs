//! Setup steps and expected values that tests in several files share

use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::outstation::database::{
    Add, BinaryInputConfig, Database, EventClass, Update, UpdateOptions,
};
use crate::outstation::tests::harness::{
    null_unsolicited, unsolicited_confirm, Event, OutstationHarness, ENABLE_UNSOLICITED_SEQ_0,
};
use crate::outstation::{BufferState, ClassCount, TypeCount};

/// expect the null unsolicited response sent at startup and confirm it
pub(crate) async fn confirm_null_unsolicited(harness: &mut OutstationHarness) {
    harness.expect_response(&null_unsolicited(0)).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
}

/// enable unsolicited responses for classes 1, 2 and 3, answered with the restart IIN
pub(crate) async fn enable_unsolicited(harness: &mut OutstationHarness) {
    harness
        .test_request_response(ENABLE_UNSOLICITED_SEQ_0, &[0xC0, 0x81, 0x80, 0x00])
        .await;
}

/// class 1 binary input 0, updated to true with an event
pub(crate) fn add_binary_event(db: &mut Database) {
    db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    db.update(
        0,
        &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
        UpdateOptions::default(),
    );
}

/// a buffer with no events
pub(crate) fn empty_buffer_state() -> BufferState {
    BufferState {
        classes: ClassCount {
            num_class_1: 0,
            num_class_2: 0,
            num_class_3: 0,
        },
        types: TypeCount {
            num_binary_input: 0,
            num_double_bit_binary_input: 0,
            num_binary_output_status: 0,
            num_counter: 0,
            num_frozen_counter: 0,
            num_analog: 0,
            num_analog_output_status: 0,
            num_octet_string: 0,
        },
    }
}

/// a buffer with `count` class 1 binary input events
pub(crate) fn class_1_binaries(count: usize) -> BufferState {
    let mut state = empty_buffer_state();
    state.classes.num_class_1 = count;
    state.types.num_binary_input = count;
    state
}
