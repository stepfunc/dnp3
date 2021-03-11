use crate::outstation::tests::harness::*;
use crate::outstation::{FreezeIndices, FreezeType};

const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const EMPTY_RESPONSE_PARAM_ERROR: &[u8] = &[0xC0, 0x81, 0x80, 0x04];
const EMPTY_RESPONSE_NO_FUNC_SUPPORTED: &[u8] = &[0xC0, 0x81, 0x80, 0x01];

#[test]
fn immediate_freeze_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(&[0xC0, 0x07, 20, 0, 0x06], EMPTY_RESPONSE);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::All, FreezeType::ImmediateFreeze),
        Event::EndControls,
    ]);
}

#[test]
fn immediate_freeze_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(&[0xC0, 0x07, 20, 0, 0x00, 0, 10], EMPTY_RESPONSE);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::Range(0, 10), FreezeType::ImmediateFreeze),
        Event::EndControls,
    ]);
}

#[test]
fn immediate_freeze_no_response_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_no_response(&[0xC0, 0x08, 20, 0, 0x06]);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::All, FreezeType::ImmediateFreeze),
        Event::EndControls,
    ]);
}

#[test]
fn immediate_freeze_no_response_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_no_response(&[0xC0, 0x08, 20, 0, 0x00, 0, 10]);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::Range(0, 10), FreezeType::ImmediateFreeze),
        Event::EndControls,
    ]);
}

#[test]
fn freeze_and_clear_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(&[0xC0, 0x09, 20, 0, 0x06], EMPTY_RESPONSE);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::All, FreezeType::FreezeAndClear),
        Event::EndControls,
    ]);
}

#[test]
fn freeze_and_clear_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(&[0xC0, 0x09, 20, 0, 0x00, 0, 10], EMPTY_RESPONSE);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::Range(0, 10), FreezeType::FreezeAndClear),
        Event::EndControls,
    ]);
}

#[test]
fn freeze_and_clear_no_response_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_no_response(&[0xC0, 0x0A, 20, 0, 0x06]);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::All, FreezeType::FreezeAndClear),
        Event::EndControls,
    ]);
}

#[test]
fn freeze_and_clear_no_response_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_no_response(&[0xC0, 0x0A, 20, 0, 0x00, 0, 10]);

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::Range(0, 10), FreezeType::FreezeAndClear),
        Event::EndControls,
    ]);
}

#[test]
fn freeze_invalid_object() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(
        &[0xC0, 0x07, 22, 0, 0x06, 20, 0, 0x06],
        EMPTY_RESPONSE_NO_FUNC_SUPPORTED,
    );

    harness.check_events(&[
        Event::BeginControls,
        Event::Freeze(FreezeIndices::All, FreezeType::ImmediateFreeze),
        Event::EndControls,
    ]);
}
