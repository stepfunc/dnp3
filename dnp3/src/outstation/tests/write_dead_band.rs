use crate::outstation::database::{Add, AnalogInputConfig, EventClass};
use crate::outstation::tests::harness::*;

const EMPTY_RESPONSE: &[u8] = &[0xC0, 0x81, 0x80, 0x00];

#[tokio::test]
async fn write_g34_var1() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(3, Some(EventClass::Class1), AnalogInputConfig::default()));

    harness
        .test_request_response(
            &[0xC0, 0x02, 0x22, 0x01, 0x17, 0x01, 0x03, 0xCA, 0xFE],
            EMPTY_RESPONSE,
        )
        .await;

    harness.check_events(&[
        Event::BeginWriteDeadBands,
        Event::WriteDeadBand(3, f64::from(0xFECAu16)),
        Event::EndWriteDeadBands,
    ]);
}

fn harness_with_analog_input() -> OutstationHarness {
    let harness = new_harness(get_default_config());
    harness
        .handle
        .database
        .transaction(|db| db.add(3, Some(EventClass::Class1), AnalogInputConfig::default()));
    harness
}

fn write_g34_var3(value: f32) -> [u8; 11] {
    let [a, b, c, d] = value.to_le_bytes();
    [0xC0, 0x02, 0x22, 0x03, 0x17, 0x01, 0x03, a, b, c, d]
}

#[tokio::test]
async fn write_g34_var3_accepts_infinity() {
    let mut harness = harness_with_analog_input();

    harness
        .test_request_response(&write_g34_var3(f32::INFINITY), EMPTY_RESPONSE)
        .await;

    harness.check_events(&[
        Event::BeginWriteDeadBands,
        Event::WriteDeadBand(3, f64::INFINITY),
        Event::EndWriteDeadBands,
    ]);
}

#[tokio::test]
async fn write_g34_var3_rejects_nan_and_negative_values() {
    let mut harness = harness_with_analog_input();

    for value in [f32::NAN, -1.0, f32::NEG_INFINITY] {
        // IIN2.2 (PARAMETER_ERROR)
        harness
            .test_request_response(&write_g34_var3(value), &[0xC0, 0x81, 0x80, 0x04])
            .await;

        harness.check_events(&[Event::BeginWriteDeadBands, Event::EndWriteDeadBands]);
    }
}

#[tokio::test]
async fn write_g34_var2() {
    let mut harness = harness_with_analog_input();

    harness
        .test_request_response(
            &[
                0xC0, 0x02, 0x22, 0x02, 0x17, 0x01, 0x03, 0x01, 0x02, 0x03, 0x04,
            ],
            EMPTY_RESPONSE,
        )
        .await;

    harness.check_events(&[
        Event::BeginWriteDeadBands,
        Event::WriteDeadBand(3, f64::from(0x04030201u32)),
        Event::EndWriteDeadBands,
    ]);
}

#[tokio::test]
async fn write_g34_with_two_byte_count_and_prefix() {
    let mut harness = harness_with_analog_input();

    // g34 var 1, 2 and 3 for index 3, each with a 2-byte count and prefix
    let [a, b, c, d] = 2.5f32.to_le_bytes();
    let requests: [&[u8]; 3] = [
        &[
            0xC0, 0x02, 0x22, 0x01, 0x28, 0x01, 0x00, 0x03, 0x00, 0xCA, 0xFE,
        ],
        &[
            0xC0, 0x02, 0x22, 0x02, 0x28, 0x01, 0x00, 0x03, 0x00, 0x01, 0x02, 0x03, 0x04,
        ],
        &[
            0xC0, 0x02, 0x22, 0x03, 0x28, 0x01, 0x00, 0x03, 0x00, a, b, c, d,
        ],
    ];
    let values = [f64::from(0xFECAu16), f64::from(0x04030201u32), 2.5];

    for (request, value) in requests.into_iter().zip(values) {
        harness.test_request_response(request, EMPTY_RESPONSE).await;

        harness.check_events(&[
            Event::BeginWriteDeadBands,
            Event::WriteDeadBand(3, value),
            Event::EndWriteDeadBands,
        ]);
    }
}
