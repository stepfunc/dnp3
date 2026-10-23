// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::variations::Group50Var2;
use crate::app::Timestamp;
use crate::outstation::tests::harness::*;
use crate::outstation::{FreezeIndices, FreezeInterval, FreezeType};
use crate::util::test_support::watchdog;

#[tokio::test]
async fn immediate_freeze_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x07, 20, 0, 0x06], &empty_response(0, 0))
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn immediate_freeze_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x07, 20, 0, 0x00, 0, 10], &empty_response(0, 0))
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn immediate_freeze_no_response_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(&[0xC0, 0x08, 20, 0, 0x06]).await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn immediate_freeze_no_response_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(&[0xC0, 0x08, 20, 0, 0x00, 0, 10])
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x09, 20, 0, 0x06], &empty_response(0, 0))
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(&[0xC0, 0x09, 20, 0, 0x00, 0, 10], &empty_response(0, 0))
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_no_response_all_counters() {
    let mut harness = new_harness(get_default_config());

    harness.send_and_process(&[0xC0, 0x0A, 20, 0, 0x06]).await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_and_clear_no_response_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(&[0xC0, 0x0A, 20, 0, 0x00, 0, 10])
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(0, 10),
        FreezeType::FreezeAndClear,
    )]);
}

#[tokio::test]
async fn freeze_invalid_object() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x07, 22, 0, 0x06, 20, 0, 0x06],
            &empty_response(0, iin2::OBJECT_UNKNOWN),
        )
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn freeze_at_time_without_g50v1() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x0B, 20, 0, 0x06],
            &empty_response(0, iin2::PARAMETER_ERROR),
        )
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn freeze_at_time() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[
                0xC0, 0x0B, 0x32, 0x02, 0x07, 0x01, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
                0xCC, 0xDD, 20, 0, 0x06,
            ],
            &empty_response(0, 0),
        )
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAtTime(FreezeInterval::PeriodicallyFreeze(
            Timestamp::new(0xFF),
            0xDDCCBBAA,
        )),
    )]);
}

#[tokio::test]
async fn freeze_at_time_no_ack() {
    let mut harness = new_harness(get_default_config());

    harness
        .send_and_process(&[
            0xC0, 0x0C, 0x32, 0x02, 0x07, 0x01, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
            0xCC, 0xDD, 20, 0, 0x06,
        ])
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::All,
        FreezeType::FreezeAtTime(FreezeInterval::PeriodicallyFreeze(
            Timestamp::new(0xFF),
            0xDDCCBBAA,
        )),
    )]);

    harness.expect_no_response();
}

#[tokio::test]
async fn immediate_freeze_two_byte_range_of_counters() {
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &[0xC0, 0x07, 20, 0, 0x01, 0x00, 0x01, 0x10, 0x01],
            &empty_response(0, 0),
        )
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(256, 272),
        FreezeType::ImmediateFreeze,
    )]);
}

#[tokio::test]
async fn freeze_at_time_with_two_byte_count() {
    let mut harness = new_harness(get_default_config());

    // g50v2 with a 2-byte count, then g20v0 with a 1-byte start/stop
    harness
        .test_request_response(
            &[
                0xC0, 0x0B, 0x32, 0x02, 0x08, 0x01, 0x00, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA,
                0xBB, 0xCC, 0xDD, 20, 0, 0x00, 3, 4,
            ],
            &empty_response(0, 0),
        )
        .await;

    harness.check_events(&[Event::Freeze(
        FreezeIndices::Range(3, 4),
        FreezeType::FreezeAtTime(FreezeInterval::PeriodicallyFreeze(
            Timestamp::new(0xFF),
            0xDDCCBBAA,
        )),
    )]);
}

#[test]
fn freeze_interval_maps_each_shape_of_time_and_interval() {
    let time = Timestamp::new(0xFF);
    let cases = [
        (Timestamp::zero(), 0, FreezeInterval::FreezeOnceImmediately),
        (time, 0, FreezeInterval::FreezeOnceAtTime(time)),
        (
            Timestamp::zero(),
            1000,
            FreezeInterval::PeriodicallyFreezeRelative(1000),
        ),
        (time, 1000, FreezeInterval::PeriodicallyFreeze(time, 1000)),
    ];

    for (time, interval, expected) in cases {
        assert_eq!(FreezeInterval::new(time, interval), expected);
        assert_eq!(expected.get_time_and_interval(), (time, interval));

        let g50v2 = Group50Var2 { time, interval };
        assert_eq!(FreezeInterval::from(g50v2), expected);
        assert_eq!(Group50Var2::from(expected), g50v2);
    }
}

#[tokio::test(start_paused = true)]
async fn freeze_at_time_with_two_time_and_interval_objects_sets_parameter_error() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());

        // g50v2 with a count of 2, then g20v0 all objects
        harness
            .test_request_response(
                &[
                    0xC0, 0x0B, 50, 2, 0x07, 2, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB,
                    0xCC, 0xDD, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD, 20, 0,
                    0x06,
                ],
                &empty_response(0, iin2::PARAMETER_ERROR),
            )
            .await;

        harness.check_no_events();
    })
    .await;
}
