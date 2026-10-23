// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The solicited and unsolicited responses the outstation keeps to send again: each resend is the
//! bytes of the response it repeats, whatever was built in either slot since

use std::ops::Range;

use crate::app::control::CommandStatus;
use crate::app::measurement::*;
use crate::app::{BufferSize, FunctionCode};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::OperateType;
use crate::outstation::BufferState;

const RESTART: u8 = 0x80;
const CLASS_1: u8 = 0x02;
const PARAMETER_ERROR: u8 = 0x04;

// control fields of solicited responses
const FIR_FIN: u8 = 0xC0;
const FIR_CON: u8 = 0xA0;
const FIN_CON: u8 = 0x60;

/// a response header and a g2v1 header leave room for this many 3-byte events in a minimum
/// fragment
const EVENTS_IN_MIN_FRAGMENT: u16 = 80;

/// a solicited response
fn response(control: u8, seq: u8, iin1: u8, iin2: u8, objects: &[u8]) -> Vec<u8> {
    let mut bytes = vec![control | seq, 0x81, iin1, iin2];
    bytes.extend_from_slice(objects);
    bytes
}

/// g52v2, the processing delay in a DELAY_MEASURE response
fn g52v2(delay: u16) -> Vec<u8> {
    let mut bytes = vec![0x34, 0x02, 0x07, 0x01];
    bytes.extend(delay.to_le_bytes());
    bytes
}

/// the echo of a g41v2 control at index 7 with `status`
fn control_echo(status: CommandStatus) -> Vec<u8> {
    vec![41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, status.as_u8()]
}

/// a SELECT of 70 g41v2 controls, whose echo doesn't fit in a minimum fragment
fn select_that_does_not_fit(seq: u8) -> Vec<u8> {
    let count = 70;
    let mut select = vec![0xC0 | seq, FunctionCode::Select.as_u8(), 41, 2, 0x17, count];
    for index in 0..count {
        select.extend_from_slice(&[index, 0x01, 0x02, 0x00]);
    }
    select
}

/// class 1 binary inputs 0 to `count - 1`, each updated to true with an event
fn add_binary_events(database: &mut DatabaseHandle, count: u16) {
    database.transaction(|db| {
        for index in 0..count {
            db.add(
                index,
                Some(EventClass::Class1),
                BinaryInputConfig::default(),
            );
        }
        for index in 0..count {
            db.update(
                index,
                &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::default(),
            );
        }
    });
}

fn set_processing_delay(harness: &OutstationHarness, delay: u16) {
    harness.application_data.lock().unwrap().processing_delay = delay;
}

/// expect the master's confirm to delete the events with `cleared` ids and leave `remaining`
fn check_confirmed(harness: &mut OutstationHarness, cleared: Range<u64>, remaining: BufferState) {
    harness.check_events(&[Event::BeginConfirm]);
    for id in cleared {
        harness.check_events(&[Event::Cleared(id)]);
    }
    harness.check_events(&[Event::EndConfirm(remaining)]);
}

/// minimum solicited fragments and room for the events of a two-fragment READ
fn min_fragment_config() -> OutstationConfig {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    config.event_buffer_config = EventBufferConfig::all_types(100);
    config
}

/// one more event than fits in the first of a series of minimum fragments
async fn two_fragment_harness() -> (OutstationHarness, Vec<u8>, Vec<u8>) {
    let mut harness = new_harness(min_fragment_config());
    let count = EVENTS_IN_MIN_FRAGMENT + 1;
    add_binary_events(&mut harness.handle.database, count);

    let first = response(FIR_CON, 0, RESTART | CLASS_1, 0, &g2v1_events(0..count - 1));
    let second = response(FIN_CON, 1, RESTART, 0, &g2v1_events(count - 1..count));
    assert_eq!(first.len(), <BufferSize>::MIN);
    (harness, first, second)
}

/// an outstation whose null unsolicited response was confirmed and that sends unsolicited
/// responses for every class
async fn enabled_unsolicited_harness(config: OutstationConfig) -> OutstationHarness {
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    harness.check_no_events();
    harness
}

/// the unsolicited response with one binary event, sent with `seq` and waiting for its confirm
async fn unsolicited_event(harness: &mut OutstationHarness, seq: u8) -> Vec<u8> {
    add_binary_events(&mut harness.handle.database, 1);
    let expected = unsolicited(seq, RESTART, &g2v1_events(0..1));
    harness.expect_response(&expected).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(seq)]);
    expected
}

/// let the unsolicited confirm wait time out and expect the response sent again
async fn expect_unsolicited_retry(harness: &mut OutstationHarness, seq: u8, expected: &[u8]) {
    harness.expect_response(expected).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(seq, true)]);
}

// ---------- retransmission bytes ----------

/// IEEE 1815-2012 Table 6-1 row 17: each repeat of the READ is the fragment awaiting its confirm
#[tokio::test]
async fn repeated_read_resends_each_fragment_of_a_series_byte_for_byte() {
    let (mut harness, first, second) = two_fragment_harness().await;
    let read = read_class_1(0);

    harness.test_request_response(&read, &first).await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.test_request_response(&read, &first).await;
    harness.check_no_events();

    harness.test_request_response(&confirm(0), &second).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);
    check_confirmed(
        &mut harness,
        0..EVENTS_IN_MIN_FRAGMENT.into(),
        class_1_binaries(1),
    );

    harness.test_request_response(&read, &second).await;
    harness.test_request_response(&read, &second).await;
    harness.check_no_events();

    harness.send_and_process(&confirm(1)).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(1)]);
    check_confirmed(
        &mut harness,
        EVENTS_IN_MIN_FRAGMENT.into()..u64::from(EVENTS_IN_MIN_FRAGMENT) + 1,
        class_1_binaries(0),
    );
    harness.check_no_events();
    harness.expect_no_response();
}

// ---------- failed and partial builds ----------

/// a partial unsolicited response, with the events that fit, is sent again unchanged, and the
/// rest follow in the next series
#[tokio::test(start_paused = true)]
async fn partial_unsolicited_response_is_resent_byte_for_byte() {
    let mut config = get_default_unsolicited_config();
    config.unsolicited_buffer_size = BufferSize::min();
    config.event_buffer_config = EventBufferConfig::all_types(100);
    let mut harness = enabled_unsolicited_harness(config).await;

    let count = EVENTS_IN_MIN_FRAGMENT + 1;
    add_binary_events(&mut harness.handle.database, count);
    let first = unsolicited(1, RESTART | CLASS_1, &g2v1_events(0..count - 1));
    assert_eq!(first.len(), <BufferSize>::MIN);
    harness.expect_response(&first).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    expect_unsolicited_retry(&mut harness, 1, &first).await;
    expect_unsolicited_retry(&mut harness, 1, &first).await;

    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    check_confirmed(&mut harness, 0..u64::from(count - 1), class_1_binaries(1));

    let second = unsolicited(2, RESTART, &g2v1_events(count - 1..count));
    harness.expect_response(&second).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
    expect_unsolicited_retry(&mut harness, 2, &second).await;
}

/// a control response that doesn't fit is an error without objects, even after a response with
/// objects, and its repeat is the same error
#[tokio::test]
async fn control_response_that_does_not_fit_is_resent_without_objects() {
    let mut harness = new_harness(min_fragment_config());
    set_processing_delay(&harness, 0x1234);

    harness
        .test_request_response(
            &delay_measure(0),
            &response(FIR_FIN, 0, RESTART, 0, &g52v2(0x1234)),
        )
        .await;

    let select = select_that_does_not_fit(1);
    let error = response(FIR_FIN, 1, RESTART, PARAMETER_ERROR, &[]);
    harness.test_request_response(&select, &error).await;
    harness.test_request_response(&select, &error).await;
    harness.check_no_events();
}

// ---------- the control-response fit probe ----------

/// the fit probe echoes every control with SUCCESS; the response is the echo of the statuses the
/// controls returned
#[tokio::test]
async fn control_response_is_the_executed_echo_not_the_fit_probe() {
    let mut harness = new_harness(get_default_config());
    harness.set_control_status(7, CommandStatus::NotSupported);

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 0, 7);
    let expected = response(
        FIR_FIN,
        0,
        RESTART,
        PARAMETER_ERROR,
        &control_echo(CommandStatus::NotSupported),
    );
    harness
        .test_request_response(&direct_operate, &expected)
        .await;
    harness.check_events(&operated(OperateType::DirectOperate));

    harness.set_control_status(7, CommandStatus::Success);
    harness
        .test_request_response(&direct_operate, &expected)
        .await;
    harness.check_no_events();
}

/// the fit probe writes into the solicited slot, so the unsolicited response it interrupts is
/// sent again unchanged
#[tokio::test(start_paused = true)]
async fn fit_probe_in_the_unsolicited_confirm_wait_leaves_the_unsolicited_response() {
    let mut config = get_default_unsolicited_config();
    config.solicited_buffer_size = BufferSize::min();
    let mut harness = enabled_unsolicited_harness(config).await;
    let unsolicited = unsolicited_event(&mut harness, 1).await;

    let select = select_that_does_not_fit(1);
    let error = response(FIR_FIN, 1, RESTART, PARAMETER_ERROR, &[]);
    harness.test_request_response(&select, &error).await;
    harness.test_request_response(&select, &error).await;
    harness.check_no_events();

    expect_unsolicited_retry(&mut harness, 1, &unsolicited).await;

    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    check_confirmed(&mut harness, 0..1, class_1_binaries(0));

    // and the error is still the response to the SELECT from idle
    harness.test_request_response(&select, &error).await;
    harness.check_no_events();
    harness.expect_no_response();
}

/// a control whose fit probe succeeds but that the handler fails, during the unsolicited confirm
/// wait: its response and repeats are the executed echo, and the unsolicited retry is unchanged
#[tokio::test(start_paused = true)]
async fn failed_control_in_the_unsolicited_confirm_wait_repeats_the_executed_echo() {
    let mut harness = enabled_unsolicited_harness(get_default_unsolicited_config()).await;
    let unsolicited = unsolicited_event(&mut harness, 1).await;
    harness.set_control_status(7, CommandStatus::HardwareError);

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 1, 7);
    let echo = response(
        FIR_FIN,
        1,
        RESTART,
        0,
        &control_echo(CommandStatus::HardwareError),
    );
    harness.test_request_response(&direct_operate, &echo).await;
    harness.check_events(&operated(OperateType::DirectOperate));

    harness.set_control_status(7, CommandStatus::Success);
    harness.test_request_response(&direct_operate, &echo).await;
    harness.check_no_events();

    expect_unsolicited_retry(&mut harness, 1, &unsolicited).await;

    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    check_confirmed(&mut harness, 0..1, class_1_binaries(0));

    // from idle, the repeat is still the executed echo
    harness.test_request_response(&direct_operate, &echo).await;
    harness.check_no_events();
    harness.expect_no_response();
}

// ---------- an empty response after a populated one ----------

/// a request that ends a series with an empty response leaves none of the fragment's objects
#[tokio::test]
async fn empty_response_after_a_fragment_has_no_objects() {
    let (mut harness, first, _) = two_fragment_harness().await;

    harness
        .test_request_response(&read_class_1(0), &first)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    // the unconfirmed events are reported again
    let empty = response(FIR_FIN, 1, RESTART | CLASS_1, 0, &[]);
    harness
        .test_request_response(&record_current_time(1), &empty)
        .await;
    harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
    harness
        .test_request_response(&record_current_time(1), &empty)
        .await;
    harness.check_no_events();

    // and the series starts again
    let again = response(
        FIR_CON,
        2,
        RESTART | CLASS_1,
        0,
        &g2v1_events(0..EVENTS_IN_MIN_FRAGMENT),
    );
    harness
        .test_request_response(&read_class_1(2), &again)
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(2)]);
}

// ---------- clear and rebuild ----------

/// a request without a response leaves nothing to repeat, and the next request builds its own
#[tokio::test]
async fn no_ack_request_clears_the_response_and_the_next_request_rebuilds_it() {
    let mut harness = new_harness(get_default_config());
    set_processing_delay(&harness, 0x1234);

    harness
        .test_request_response(
            &delay_measure(0),
            &response(FIR_FIN, 0, RESTART, 0, &g52v2(0x1234)),
        )
        .await;

    let no_ack = g41v2_control(FunctionCode::DirectOperateNoResponse, 1, 7);
    harness.send_and_process(&no_ack).await;
    harness.check_events(&operated(OperateType::DirectOperateNoAck));
    harness.expect_no_response();
    harness.send_and_process(&no_ack).await;
    harness.check_no_events();
    harness.expect_no_response();

    // the DELAY_MEASURE isn't the last request any more, so it's answered again
    set_processing_delay(&harness, 0x5678);
    let rebuilt = response(FIR_FIN, 0, RESTART, 0, &g52v2(0x5678));
    harness
        .test_request_response(&delay_measure(0), &rebuilt)
        .await;

    set_processing_delay(&harness, 0x9ABC);
    harness
        .test_request_response(&delay_measure(0), &rebuilt)
        .await;
    harness.check_no_events();
}

/// a new connection keeps neither response: the unsolicited events go in a new series, and the
/// solicited request is answered again
#[tokio::test]
async fn reconnect_rebuilds_both_responses() {
    let mut harness = enabled_unsolicited_harness(get_default_unsolicited_config()).await;
    set_processing_delay(&harness, 0x1234);
    harness
        .test_request_response(
            &delay_measure(1),
            &response(FIR_FIN, 1, RESTART, 0, &g52v2(0x1234)),
        )
        .await;
    unsolicited_event(&mut harness, 1).await;

    let mut harness = harness.drop_link().await.reconnect();
    set_processing_delay(&harness, 0x5678);

    harness
        .expect_response(&unsolicited(2, RESTART, &g2v1_events(0..1)))
        .await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);

    harness
        .test_request_response(
            &delay_measure(1),
            &response(FIR_FIN, 1, RESTART, 0, &g52v2(0x5678)),
        )
        .await;
    harness.check_no_events();
}

// ---------- interleaved solicited and unsolicited slots ----------

/// solicited responses with objects, built during the unsolicited confirm wait, are repeated
/// from their own slot, and the unsolicited retry from its own
#[tokio::test(start_paused = true)]
async fn solicited_responses_in_the_unsolicited_confirm_wait_leave_the_unsolicited_response() {
    let mut harness = enabled_unsolicited_harness(get_default_unsolicited_config()).await;
    set_processing_delay(&harness, 0x1234);
    let unsolicited = unsolicited_event(&mut harness, 1).await;

    let delay = response(FIR_FIN, 1, RESTART, 0, &g52v2(0x1234));
    harness
        .test_request_response(&delay_measure(1), &delay)
        .await;
    set_processing_delay(&harness, 0x5678);
    harness
        .test_request_response(&delay_measure(1), &delay)
        .await;

    let direct_operate = g41v2_control(FunctionCode::DirectOperate, 2, 7);
    let echo = response(
        FIR_FIN,
        2,
        RESTART,
        0,
        &control_echo(CommandStatus::Success),
    );
    harness.test_request_response(&direct_operate, &echo).await;
    harness.check_events(&operated(OperateType::DirectOperate));
    harness.test_request_response(&direct_operate, &echo).await;
    harness.check_no_events();

    expect_unsolicited_retry(&mut harness, 1, &unsolicited).await;

    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    check_confirmed(&mut harness, 0..1, class_1_binaries(0));

    // from idle, the repeat is the stored echo
    harness.test_request_response(&direct_operate, &echo).await;
    harness.check_no_events();
    harness.expect_no_response();
}

/// an unsolicited response built after a solicited one leaves the solicited response to repeat
#[tokio::test(start_paused = true)]
async fn unsolicited_response_in_idle_leaves_the_solicited_response() {
    let mut harness = enabled_unsolicited_harness(get_default_unsolicited_config()).await;
    set_processing_delay(&harness, 0x1234);

    let delay = response(FIR_FIN, 1, RESTART, 0, &g52v2(0x1234));
    harness
        .test_request_response(&delay_measure(1), &delay)
        .await;
    set_processing_delay(&harness, 0x5678);

    let unsolicited = unsolicited_event(&mut harness, 1).await;

    // during the unsolicited confirm wait
    harness
        .test_request_response(&delay_measure(1), &delay)
        .await;
    expect_unsolicited_retry(&mut harness, 1, &unsolicited).await;

    harness.send_and_process(&unsolicited_confirm(1)).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    check_confirmed(&mut harness, 0..1, class_1_binaries(0));

    // and from idle
    harness
        .test_request_response(&delay_measure(1), &delay)
        .await;
    harness.check_no_events();
    harness.expect_no_response();
}
