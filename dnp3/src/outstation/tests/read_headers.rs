// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::app::measurement::*;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;

/// READ of four headers, two of them unsupported:
/// g1v2 8-bit start/stop 0 to 1, g13v1 all objects, g2v0 8-bit count of 1, g111v1 8-bit count of 1
const READ_SUPPORTED_AND_UNSUPPORTED: &[u8] = &[
    0xC0, 0x01, 0x01, 0x02, 0x00, 0x00, 0x01, 0x0D, 0x01, 0x06, 0x02, 0x00, 0x07, 0x01, 0x6F, 0x01,
    0x07, 0x01,
];

/// FIR, FIN, CON, seq 0, class 1 events, OBJECT_UNKNOWN: the first binary input event, then both
/// binary inputs
const SUPPORTED_RESPONSE: &[u8] = &[
    0xE0, 0x81, 0x82, 0x02, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81, 0x01, 0x02, 0x01, 0x00,
    0x00, 0x01, 0x00, 0x81, 0x81,
];

fn add_binary_inputs_with_events(db: &mut Database) {
    for index in [0, 1] {
        db.add(
            index,
            Some(EventClass::Class1),
            BinaryInputConfig::default(),
        );
        db.update(
            index,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    }
}

#[tokio::test]
async fn read_selects_its_supported_headers_and_reports_the_unsupported_ones() {
    let mut harness = new_harness(get_default_config());
    harness
        .handle
        .database
        .transaction(add_binary_inputs_with_events);
    harness
        .test_request_response(READ_SUPPORTED_AND_UNSUPPORTED, SUPPORTED_RESPONSE)
        .await;
}

#[tokio::test]
async fn deferred_read_selects_its_supported_headers_and_reports_the_unsupported_ones() {
    let mut harness = new_harness(get_default_unsolicited_config());

    harness.expect_response(&null_unsolicited(0)).await;
    harness.send_and_process(&unsolicited_confirm(0)).await;
    harness
        .test_request_response(ENABLE_UNSOLICITED_SEQ_0, &empty_response(0, 0))
        .await;
    harness
        .handle
        .database
        .transaction(add_binary_inputs_with_events);
    let _unsolicited = harness.expect_write().await;

    // deferred until the unsolicited confirm wait ends
    harness
        .send_and_process(READ_SUPPORTED_AND_UNSUPPORTED)
        .await;
    harness.expect_no_response();

    tokio::time::pause();
    harness
        .wait_for_events(&[
            Event::EnterUnsolicitedConfirmWait(0),
            Event::UnsolicitedConfirmReceived(0),
            Event::EnterUnsolicitedConfirmWait(1),
            Event::UnsolicitedConfirmTimeout(1, false),
        ])
        .await;
    harness.expect_response(SUPPORTED_RESPONSE).await;
}

#[tokio::test]
async fn read_of_a_variation_the_qualifier_cannot_carry_sets_parameter_error() {
    let mut harness = new_harness(get_default_config());
    harness
        .handle
        .database
        .transaction(add_binary_inputs_with_events);

    let requests: &[&[u8]] = &[
        // g32v0, 8-bit start/stop, 0 to 1
        &[0xC0, 0x01, 0x20, 0x00, 0x00, 0x00, 0x01],
        // g2v1, 8-bit start/stop, 0 to 1
        &[0xC0, 0x01, 0x02, 0x01, 0x00, 0x00, 0x01],
        // g1v2, 8-bit count of 1
        &[0xC0, 0x01, 0x01, 0x02, 0x07, 0x01],
        // g50v1, all objects
        &[0xC0, 0x01, 0x32, 0x01, 0x06],
        // g1v2, 8-bit start/stop, 1 to 0
        &[0xC0, 0x01, 0x01, 0x02, 0x00, 0x01, 0x00],
        // a supported header, then g2v1 with an 8-bit start/stop
        &[
            0xC0, 0x01, 0x01, 0x02, 0x00, 0x00, 0x01, 0x02, 0x01, 0x00, 0x00, 0x01,
        ],
    ];

    for request in requests {
        harness
            .test_request_response(
                request,
                &solicited(
                    0,
                    false,
                    iin1::RESTART | iin1::CLASS_1,
                    iin2::PARAMETER_ERROR,
                ),
            )
            .await;
    }
}
