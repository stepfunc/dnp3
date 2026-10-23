// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use crate::app::Timestamp;
use crate::outstation::RequestError;
use crate::util::test_support::watchdog;

use super::harness::*;

const RESPONSE_TIME_DELAY_FINE_CAFE: &[u8] =
    &[0xC0, 0x81, 0x80, 0x00, 0x34, 0x02, 0x07, 0x01, 0xFE, 0xCA];
const WRITE_ABSOLUTE_TIME: &[u8] = &[
    0xC1, 0x02, 50, 1, 0x07, 1, 0xC0, 0x24, 0x0E, 0xDA, 0x77, 0x01,
];

const WRITE_LAST_RECORDED_TIME: &[u8] = &[
    0xC1, 0x02, 50, 3, 0x07, 1, 0xFE, 0xCA, 0x00, 0x00, 0x00, 0x00,
];

#[tokio::test]
async fn responds_to_delay_measure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness
        .test_request_response(&delay_measure(0), RESPONSE_TIME_DELAY_FINE_CAFE)
        .await;
}

#[tokio::test]
async fn non_lan_procedure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness
        .test_request_response(&delay_measure(0), RESPONSE_TIME_DELAY_FINE_CAFE)
        .await;
    harness
        .test_request_response(WRITE_ABSOLUTE_TIME, &empty_response(1, 0))
        .await;

    harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(1614271096000))]);
}

#[tokio::test(start_paused = true)]
async fn lan_procedure() {
    let mut harness = new_harness(get_default_config());

    harness.application_data.lock().unwrap().processing_delay = 0xCAFE;

    harness
        .test_request_response(&record_current_time(0), &empty_response(0, 0))
        .await;
    harness
        .test_request_response(WRITE_LAST_RECORDED_TIME, &empty_response(1, 0))
        .await;

    harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(0xCAFE))]);
}

#[tokio::test(start_paused = true)]
async fn lan_procedure_with_two_byte_count() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&record_current_time(0), &empty_response(0, 0))
        .await;
    harness
        .test_request_response(
            &[
                0xC1, 0x02, 50, 3, 0x08, 1, 0, 0xFE, 0xCA, 0x00, 0x00, 0x00, 0x00,
            ],
            &empty_response(1, 0),
        )
        .await;

    harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(0xCAFE))]);
}

/// WRITE of `count` g50v1 (absolute time) objects, each `time`, with a 1-byte count
fn write_absolute_time(seq: u8, count: u8, time: u64) -> Vec<u8> {
    write_time_objects(seq, 1, count, time)
}

/// WRITE of `count` g50v3 (last recorded time) objects, each `time`, with a 1-byte count
fn write_last_recorded_time(seq: u8, count: u8, time: u64) -> Vec<u8> {
    write_time_objects(seq, 3, count, time)
}

fn write_time_objects(seq: u8, variation: u8, count: u8, time: u64) -> Vec<u8> {
    let mut bytes = vec![0xC0 | seq, 0x02, 50, variation, 0x07, count];
    for _ in 0..count {
        bytes.extend_from_slice(&time.to_le_bytes()[..6]);
    }
    bytes
}

const RECORDED_TIME: u64 = 1_000_000;

/// RECORD_CURRENT_TIME with sequence `seq`, then wait `delay`
async fn record_and_wait(harness: &mut OutstationHarness, seq: u8, delay: Duration) {
    harness
        .test_request_response(&record_current_time(seq), &empty_response(seq, 0))
        .await;
    tokio::time::advance(delay).await;
}

#[tokio::test(start_paused = true)]
async fn lan_procedure_adds_the_time_elapsed_since_record_current_time() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        record_and_wait(&mut harness, 0, Duration::from_millis(1234)).await;
        harness
            .test_request_response(
                &write_last_recorded_time(1, 1, RECORDED_TIME),
                &empty_response(1, 0),
            )
            .await;

        harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(1_001_234))]);
        harness.check_no_events();
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn last_recorded_time_is_used_once() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        record_and_wait(&mut harness, 0, Duration::from_millis(10)).await;
        harness
            .test_request_response(
                &write_last_recorded_time(1, 1, RECORDED_TIME),
                &empty_response(1, 0),
            )
            .await;
        harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::new(1_000_010))]);

        // a second g50v3, with a new sequence number and time, needs a new RECORD_CURRENT_TIME
        harness
            .test_request_response(
                &write_last_recorded_time(2, 1, 2 * RECORDED_TIME),
                &empty_response(2, iin2::PARAMETER_ERROR),
            )
            .await;
        harness.check_no_events();
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_last_recorded_time_without_record_current_time_sets_parameter_error() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        harness
            .test_request_response(
                &write_last_recorded_time(0, 1, RECORDED_TIME),
                &empty_response(0, iin2::PARAMETER_ERROR),
            )
            .await;
        harness.check_no_events();
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn writes_of_time_with_a_count_other_than_one_set_parameter_error() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        let mut seq: u8 = 0;
        for count in [0, 2] {
            harness
                .test_request_response(
                    &write_absolute_time(seq, count, RECORDED_TIME),
                    &empty_response(seq, iin2::PARAMETER_ERROR),
                )
                .await;
            seq = seq.wrapping_add(1);

            // with a recorded time, so the count is the only problem
            record_and_wait(&mut harness, seq, Duration::from_millis(10)).await;
            seq = seq.wrapping_add(1);
            harness
                .test_request_response(
                    &write_last_recorded_time(seq, count, RECORDED_TIME),
                    &empty_response(seq, iin2::PARAMETER_ERROR),
                )
                .await;
            seq = seq.wrapping_add(1);
        }
        harness.check_no_events();
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn lan_procedure_accepts_the_largest_time_and_rejects_one_past_it() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        // 1234 ms after the largest 48-bit time less 1234 ms is the largest time
        record_and_wait(&mut harness, 0, Duration::from_millis(1234)).await;
        harness
            .test_request_response(
                &write_last_recorded_time(1, 1, Timestamp::MAX_VALUE - 1234),
                &empty_response(1, 0),
            )
            .await;
        harness.check_events(&[Event::WriteAbsoluteTime(Timestamp::max())]);

        // one more millisecond overflows the 48-bit time
        record_and_wait(&mut harness, 2, Duration::from_millis(1234)).await;
        harness
            .test_request_response(
                &write_last_recorded_time(3, 1, Timestamp::MAX_VALUE - 1233),
                &empty_response(3, iin2::PARAMETER_ERROR),
            )
            .await;
        harness.check_no_events();
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn lan_procedure_reports_the_error_of_the_application() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        let mut seq: u8 = 0;
        for (err, iin2) in [
            (RequestError::ParameterError, iin2::PARAMETER_ERROR),
            (RequestError::NotSupported, iin2::OBJECT_UNKNOWN),
        ] {
            harness.application_data.lock().unwrap().request_error = Some(err);
            record_and_wait(&mut harness, seq, Duration::from_millis(10)).await;
            seq = seq.wrapping_add(1);
            harness
                .test_request_response(
                    &write_last_recorded_time(seq, 1, RECORDED_TIME),
                    &empty_response(seq, iin2),
                )
                .await;
            seq = seq.wrapping_add(1);
        }
        harness.check_no_events();
    })
    .await;
}
