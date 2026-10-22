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
    harness_with_deadband(0.0)
}

fn harness_with_deadband(deadband: f64) -> OutstationHarness {
    let harness = new_harness(get_default_config());
    let config = AnalogInputConfig {
        deadband,
        ..Default::default()
    };
    harness
        .handle
        .database
        .transaction(|db| db.add(3, Some(EventClass::Class1), config));
    harness
}

fn write_g34_var3(value: f32) -> [u8; 11] {
    let [a, b, c, d] = value.to_le_bytes();
    [0xC0, 0x02, 0x22, 0x03, 0x17, 0x01, 0x03, a, b, c, d]
}

/// READ g34 `var` for index 3 with sequence number 1, and expect `value`
async fn read_g34(harness: &mut OutstationHarness, var: u8, value: &[u8]) {
    let request = [0xC1, 0x01, 0x22, var, 0x06];
    let header = [
        0xC1, 0x81, 0x80, 0x00, 0x22, var, 0x01, 0x03, 0x00, 0x03, 0x00,
    ];
    harness
        .test_request_response(&request, &[&header[..], value].concat())
        .await;
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

    // infinity in g34v3, the maximum in the integer variations
    read_g34(&mut harness, 3, &f32::INFINITY.to_le_bytes()).await;
    read_g34(&mut harness, 1, &[0xFF, 0xFF]).await;
    read_g34(&mut harness, 2, &[0xFF, 0xFF, 0xFF, 0xFF]).await;
}

#[tokio::test]
async fn write_g34_var3_accepts_a_positive_subnormal_value() {
    let mut harness = harness_with_analog_input();
    let subnormal = f32::from_bits(1);

    harness
        .test_request_response(&write_g34_var3(subnormal), EMPTY_RESPONSE)
        .await;

    harness.check_events(&[
        Event::BeginWriteDeadBands,
        Event::WriteDeadBand(3, f64::from(subnormal)),
        Event::EndWriteDeadBands,
    ]);

    read_g34(&mut harness, 3, &[0x01, 0x00, 0x00, 0x00]).await;
}

#[tokio::test]
async fn write_g34_var3_stores_negative_values_as_zero() {
    for value in [-1.0, f32::NEG_INFINITY] {
        let mut harness = harness_with_deadband(2.5);

        harness
            .test_request_response(&write_g34_var3(value), EMPTY_RESPONSE)
            .await;

        harness.check_events(&[
            Event::BeginWriteDeadBands,
            Event::WriteDeadBand(3, 0.0),
            Event::EndWriteDeadBands,
        ]);

        read_g34(&mut harness, 3, &[0x00, 0x00, 0x00, 0x00]).await;
    }
}

#[tokio::test]
async fn write_g34_var3_rejects_nan() {
    let mut harness = harness_with_deadband(2.5);

    // IIN2.2 (PARAMETER_ERROR)
    harness
        .test_request_response(&write_g34_var3(f32::NAN), &[0xC0, 0x81, 0x80, 0x04])
        .await;

    harness.check_events(&[Event::BeginWriteDeadBands, Event::EndWriteDeadBands]);

    // the deadband is unchanged
    read_g34(&mut harness, 3, &2.5f32.to_le_bytes()).await;
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
