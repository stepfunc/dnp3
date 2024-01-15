use crate::app::gen::prefixed::PrefixedVariation;
use crate::app::measurement::*;
use crate::app::parse::parser::{HeaderDetails, ParsedFragment};
use crate::app::{BufferSize, Timestamp};
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::{BufferState, ClassCount, TypeCount};

use super::harness::*;

const fn uns_confirm(seq: u8) -> u8 {
    0b1101_0000 | seq
}

const NULL_UNSOL_SEQ_0: &[u8] = &[0xF0, 0x82, 0x80, 0x00];
const NULL_UNSOL_SEQ_1: &[u8] = &[0xF1, 0x82, 0x80, 0x00];
const UNS_CONFIRM_SEQ_0: &[u8] = &[uns_confirm(0), 0x00];
const UNS_CONFIRM_SEQ_1: &[u8] = &[uns_confirm(1), 0x00];
const UNSOL_G2V1_SEQ1: &[u8] = &[
    0xF1, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const UNSOL_G2V1_SEQ2: &[u8] = &[
    0xF2, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];
const ENABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];
const DISABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x15, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];

const READ_CLASS_0: &[u8] = &[0xC0, 0x01, 0x3C, 0x01, 0x06];
const EMPTY_RESPONSE_SEQ0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const CLASS_0_RESPONSE_SEQ0: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];
const CLASS_0_RESPONSE_SEQ0_WITH_PENDING_EVENTS: &[u8] = &[
    0xC0, 0x81, 0x82, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];

fn generate_binary_event(handle: &mut DatabaseHandle) {
    handle.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        )
    });
}

async fn enable_unsolicited(harness: &mut OutstationHarness) {
    harness
        .test_request_response(ENABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
        .await;
}

async fn confirm_null_unsolicited(harness: &mut OutstationHarness) {
    harness.expect_response(NULL_UNSOL_SEQ_0).await;
    harness.send_and_process(UNS_CONFIRM_SEQ_0).await;
    harness.check_events(&[
        Event::EnterUnsolicitedConfirmWait(0),
        Event::UnsolicitedConfirmReceived(0),
    ]);
}

fn config_with_limited_retries(retries: usize) -> OutstationConfig {
    let mut config = get_default_unsolicited_config();
    config.max_unsolicited_retries = Some(retries);
    config
}

#[tokio::test]
async fn null_unsolicited_always_retries() {
    let mut harness = new_harness(get_default_unsolicited_config());
    harness.expect_response(NULL_UNSOL_SEQ_0).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

    tokio::time::pause(); //auto advance timer

    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(0, false)])
        .await;
    harness.expect_response(NULL_UNSOL_SEQ_1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
}

#[tokio::test]
async fn unsolicited_can_time_out_and_retry() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    generate_binary_event(&mut harness.handle.database);

    // first response
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // this would go on forever, but let's just test 3 iterations
    for _ in 0..3 {
        tokio::time::pause();
        harness.expect_response(UNSOL_G2V1_SEQ1).await;
        harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
        tokio::time::resume();
    }
}

#[tokio::test]
async fn unsolicited_can_timeout_and_not_retry() {
    let mut harness = new_harness(config_with_limited_retries(2));
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    generate_binary_event(&mut harness.handle.database);

    // response
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // first retry
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::resume();

    // second retry
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::UnsolicitedConfirmTimeout(1, true)]);
    tokio::time::resume();

    // timeout
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    tokio::time::resume();

    // new series
    tokio::time::pause();
    harness.expect_response(UNSOL_G2V1_SEQ2).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
}

#[tokio::test]
async fn unsolicited_can_timeout_series_wait_and_start_another_series() {
    let mut harness = new_harness(config_with_limited_retries(0));
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;
    generate_binary_event(&mut harness.handle.database);

    // first response series
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;

    harness.expect_response(UNSOL_G2V1_SEQ2).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(2)]);
}

#[tokio::test]
async fn data_unsolicited_can_be_confirmed() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    harness.send_and_process(UNS_CONFIRM_SEQ_1).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[tokio::test]
async fn defers_read_during_unsol_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read which will be deferred
    harness.send_and_process(READ_CLASS_0).await;
    harness.check_no_events();
    // now send the confirm
    harness.send_and_process(UNS_CONFIRM_SEQ_1).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
    harness.expect_response(CLASS_0_RESPONSE_SEQ0).await;
    harness.check_events(&[
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(BufferState {
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
        }),
    ]);
    harness.check_no_events();
}

#[tokio::test]
async fn defers_read_during_unsol_confirm_wait_timeout() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a read which will be deferred
    harness.send_and_process(READ_CLASS_0).await;
    harness.check_no_events();

    // expire the unsolicited response
    tokio::time::pause();
    harness
        .wait_for_events(&[Event::UnsolicitedConfirmTimeout(1, false)])
        .await;
    harness
        .expect_response(CLASS_0_RESPONSE_SEQ0_WITH_PENDING_EVENTS)
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn handles_non_read_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a delay measurement request while still in unsolicited confirm wait
    harness
        .test_request_response(
            super::data::DELAY_MEASURE,
            super::data::RESPONSE_TIME_DELAY_FINE_ZERO,
        )
        .await;
    harness.check_no_events();

    // now send the confirm
    harness.send_and_process(UNS_CONFIRM_SEQ_1).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[tokio::test]
async fn handles_invalid_request_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);
    // send a delay measurement request while still in unsolicited confirm wait
    harness
        .test_request_response(
            &[0xC0, 0x70],             // Invalid request
            &[0xC0, 0x81, 0x80, 0x01], // NO_FUNC_CODE_SUPPORT
        )
        .await;
    harness.check_no_events();

    // now send the confirm
    harness.send_and_process(UNS_CONFIRM_SEQ_1).await;
    harness.check_events(&[Event::UnsolicitedConfirmReceived(1)]);
}

#[tokio::test]
async fn handles_disable_unsolicited_during_unsolicited_confirm_wait() {
    let mut harness = new_harness(get_default_unsolicited_config());
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    generate_binary_event(&mut harness.handle.database);
    harness.expect_response(UNSOL_G2V1_SEQ1).await;
    harness.check_events(&[Event::EnterUnsolicitedConfirmWait(1)]);

    // send a disable unsolicited request
    harness
        .test_request_response(DISABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
        .await;
    harness.check_no_events();

    // check that no other unsolicited responses are sent
    tokio::time::pause();
    harness.check_no_events();
}

#[tokio::test]
async fn sends_unsolicited_from_one_update() {
    let mut config = get_default_unsolicited_config();
    config.unsolicited_buffer_size = BufferSize::min();
    config.event_buffer_config.max_analog = 1001;
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    harness.handle.database.transaction(|db| {
        for i in 0..1000 {
            db.add(
                i,
                Some(EventClass::Class1),
                AnalogInputConfig {
                    s_var: StaticAnalogInputVariation::Group30Var1,
                    e_var: EventAnalogInputVariation::Group32Var1, // 5  bytes
                    deadband: 0.0,
                },
            );
        }
    });

    harness.handle.database.transaction(|db| {
        let value = AnalogInput {
            value: 98.6,
            flags: Flags { value: 0 },
            time: None,
        };

        for i in 0..1000 {
            db.update(i, &value, UpdateOptions::detect_event());
        }
    });

    let mut total_fragments: usize = 0;
    let mut total_events = 0;

    loop {
        let rx = harness.expect_write().await;
        let response = ParsedFragment::parse(&rx).unwrap().to_response().unwrap();
        let mut num_events = 0;
        match response.objects.unwrap().get_only_header().unwrap().details {
            HeaderDetails::TwoByteCountAndPrefix(_, PrefixedVariation::Group32Var1(seq)) => {
                num_events += seq.iter().count()
            }
            x => panic!("Unexpected header: {x:?}"),
        }

        total_fragments += 1;
        total_events += num_events;

        // confirm the fragment
        let confirm = &[uns_confirm(response.header.control.seq.value()), 0x00];
        harness.send_and_process(confirm).await;

        // terminate when the outstation indicates there are no more events
        if !response.header.iin.iin1.get_class_1_events() {
            break;
        }
    }

    assert_eq!(total_events, 1000);
    assert!(total_fragments > 10);
}

#[tokio::test]
async fn buffer_overflow_issue() {
    let mut config = get_default_unsolicited_config();
    config.event_buffer_config = EventBufferConfig::all_types(1);
    let mut harness = new_harness(config);
    confirm_null_unsolicited(&mut harness).await;
    enable_unsolicited(&mut harness).await;

    fn generate_overflow(database: &mut DatabaseHandle) {
        database.transaction(|database| {
            database.update(
                0,
                &BinaryInput::new(true, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
                UpdateOptions::default(),
            );
            database.update(
                0,
                &BinaryInput::new(false, Flags::ONLINE, Time::Synchronized(Timestamp::new(0))),
                UpdateOptions::default(),
            );
        });
    }

    // Add the point
    harness.handle.database.transaction(|database| {
        database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    });

    generate_overflow(&mut harness.handle.database);

    // Unsolicited response with event data and EVENT_BUFFER_OVERFLOW
    harness
        .expect_response(&[
            0xF1, 0x82, 0x80, 0x08, // DEVICE_RESTART and EVENT_BUFFER_OVERFLOW asserted
            0x02, 0x01, 0x28, 0x01, 0x00, // 1 event g2v1 only
            0x00, 0x00, 0x01,
        ])
        .await;

    // THIS USED TO GENERATE A SUBTRACT OVERFLOW IN THE EVENT BUFFER (the panic occurred in the next line)
    generate_overflow(&mut harness.handle.database);

    harness.send_and_process(&[0xD1, 0x00]).await;

    // New unsolicited response with a single event
    harness
        .expect_response(&[
            0xF2, 0x82, 0x80, 0x08, // DEVICE_RESTART and EVENT_BUFFER_OVERFLOW asserted
            0x02, 0x01, 0x28, 0x01, 0x00, // 1 event g2v1 only
            0x00, 0x00, 0x01,
        ])
        .await;
    harness.send_and_process(&[0xD2, 0x00]).await;

    // Integrity poll response should not contain EVENT_BUFFER_OVERFLOW flag anymore
    harness
        .test_request_response(
            &[
                0xC0, 0x01, 60, 2, 0x06, 60, 3, 0x06, 60, 4, 0x06, 60, 1, 0x06,
            ],
            &[
                0xC0, 0x81, 0x80, 0x00, // Only DEVICE_RESTART
                0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, // g1v1 [0, 0]
                0x00, // Current value
            ],
        )
        .await;
}
