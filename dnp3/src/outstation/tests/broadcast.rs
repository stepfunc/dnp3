// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! What the functions an outstation accepts in a broadcast request do. A broadcast never gets a
//! response, but sets IIN1.0 in the next one. When IIN1.0 ends, broadcasts in the confirm waits and
//! broadcast ENABLE and DISABLE_UNSOLICITED are checked by the session property in
//! `session_model.rs`.

use crate::app::{FunctionCode, Timestamp};
use crate::link::header::BroadcastConfirmMode;
use crate::outstation::config::Feature;
use crate::outstation::traits::BroadcastAction;
use crate::outstation::{FreezeIndices, FreezeInterval, FreezeType};
use crate::util::test_support::watchdog;

use super::harness::*;

/// send `request` to the broadcast address that doesn't require a confirm
async fn send_broadcast(harness: &mut OutstationHarness, request: &[u8]) {
    harness.set_broadcast(Some(BroadcastConfirmMode::NotRequired));
    harness.send_and_process(request).await;
    harness.set_broadcast(None);
}

/// The response to a RECORD_CURRENT_TIME with `seq` has IIN1.0 and `iin1`. It's the next thing the
/// outstation sends, so the broadcast before it got no response.
async fn expect_broadcast_iin(harness: &mut OutstationHarness, seq: u8, iin1: u8) {
    harness
        .test_request_response(
            &record_current_time(seq),
            &solicited(seq, false, iin1::BROADCAST | iin1, 0),
        )
        .await;
    harness.check_no_events();
}

struct Case {
    broadcast: Feature,
    request: &'static [u8],
    /// the callbacks, ending with `broadcast_received`
    events: &'static [Event],
    /// IIN1 of the next response, besides IIN1.0
    iin1: u8,
}

const fn processed(function: FunctionCode) -> Event {
    Event::BroadcastReceived(function, BroadcastAction::Processed)
}

const CASES: &[Case] = &[
    // WRITE of the absolute time
    Case {
        broadcast: Feature::Enabled,
        request: &[
            0xC0, 0x02, 50, 1, 0x07, 1, 0xC0, 0x24, 0x0E, 0xDA, 0x77, 0x01,
        ],
        events: &[
            Event::WriteAbsoluteTime(Timestamp::new(0x0177_DA0E_24C0)),
            processed(FunctionCode::Write),
        ],
        iin1: iin1::RESTART,
    },
    // WRITE of IIN1.7 = 0, which clears the restart IIN
    Case {
        broadcast: Feature::Enabled,
        request: &[0xC0, 0x02, 80, 1, 0x00, 7, 7, 0x00],
        events: &[Event::ClearRestartIIN, processed(FunctionCode::Write)],
        iin1: 0,
    },
    // IMMEDIATE_FREEZE_NR of all counters
    Case {
        broadcast: Feature::Enabled,
        request: &[0xC0, 0x08, 20, 0, 0x06],
        events: &[
            Event::Freeze(FreezeIndices::All, FreezeType::ImmediateFreeze),
            processed(FunctionCode::ImmediateFreezeNoResponse),
        ],
        iin1: iin1::RESTART,
    },
    // FREEZE_CLEAR_NR of counters 0 to 10
    Case {
        broadcast: Feature::Enabled,
        request: &[0xC0, 0x0A, 20, 0, 0x00, 0, 10],
        events: &[
            Event::Freeze(FreezeIndices::Range(0, 10), FreezeType::FreezeAndClear),
            processed(FunctionCode::FreezeClearNoResponse),
        ],
        iin1: iin1::RESTART,
    },
    // FREEZE_AT_TIME_NR of all counters
    Case {
        broadcast: Feature::Enabled,
        request: &[
            0xC0, 0x0C, 50, 2, 0x07, 1, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0xAA, 0xBB, 0xCC, 0xDD,
            20, 0, 0x06,
        ],
        events: &[
            Event::Freeze(
                FreezeIndices::All,
                FreezeType::FreezeAtTime(FreezeInterval::PeriodicallyFreeze(
                    Timestamp::new(0xFF),
                    0xDDCC_BBAA,
                )),
            ),
            processed(FunctionCode::FreezeAtTimeNoResponse),
        ],
        iin1: iin1::RESTART,
    },
    // IMMEDIATE_FREEZE_NR whose object header is cut short
    Case {
        broadcast: Feature::Enabled,
        request: &[0xC0, 0x08, 20, 0],
        events: &[Event::BroadcastReceived(
            FunctionCode::ImmediateFreezeNoResponse,
            BroadcastAction::BadObjectHeaders,
        )],
        iin1: iin1::RESTART,
    },
    // IMMEDIATE_FREEZE_NR of all counters, with broadcast support disabled
    Case {
        broadcast: Feature::Disabled,
        request: &[0xC0, 0x08, 20, 0, 0x06],
        events: &[Event::BroadcastReceived(
            FunctionCode::ImmediateFreezeNoResponse,
            BroadcastAction::IgnoredByConfiguration,
        )],
        iin1: iin1::RESTART,
    },
];

#[tokio::test]
async fn broadcast_requests_have_their_effect_without_a_response() {
    watchdog(async {
        for case in CASES {
            let mut config = get_default_config();
            config.features.broadcast = case.broadcast;
            let mut harness = new_harness(config);

            send_broadcast(&mut harness, case.request).await;
            harness.check_events(case.events);
            harness.check_no_events();

            expect_broadcast_iin(&mut harness, 1, case.iin1).await;
        }
    })
    .await;
}
