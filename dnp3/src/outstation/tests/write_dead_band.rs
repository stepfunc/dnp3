// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::outstation::database::{Add, AnalogInputConfig, EventClass};
use crate::outstation::tests::harness::*;

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
            &empty_response(0, 0),
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
        .test_request_response(&write_g34_var3(f32::INFINITY), &empty_response(0, 0))
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
        .test_request_response(&write_g34_var3(subnormal), &empty_response(0, 0))
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
            .test_request_response(&write_g34_var3(value), &empty_response(0, 0))
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
            &empty_response(0, 0),
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
        harness
            .test_request_response(request, &empty_response(0, 0))
            .await;

        harness.check_events(&[
            Event::BeginWriteDeadBands,
            Event::WriteDeadBand(3, value),
            Event::EndWriteDeadBands,
        ]);
    }
}

/// the objects of a WRITE of a g34 variation for indices 3 and 4, and the deadbands they set
struct DeadBandObjects {
    variation: u8,
    first: &'static [u8],
    second: &'static [u8],
    deadbands: [f64; 2],
}

impl DeadBandObjects {
    /// the WRITE with each qualifier that Subset Level 4 requires
    fn requests(&self) -> [Vec<u8>; 4] {
        let with_header = |header: &[u8], first_prefix: &[u8], second_prefix: &[u8]| {
            let mut request = vec![0xC0, 0x02, 0x22, self.variation];
            request.extend_from_slice(header);
            request.extend_from_slice(first_prefix);
            request.extend_from_slice(self.first);
            request.extend_from_slice(second_prefix);
            request.extend_from_slice(self.second);
            request
        };

        [
            // 1-byte start-stop
            with_header(&[0x00, 0x03, 0x04], &[], &[]),
            // 2-byte start-stop
            with_header(&[0x01, 0x03, 0x00, 0x04, 0x00], &[], &[]),
            // 1-byte count and prefix
            with_header(&[0x17, 0x02], &[0x03], &[0x04]),
            // 2-byte count and prefix
            with_header(&[0x28, 0x02, 0x00], &[0x03, 0x00], &[0x04, 0x00]),
        ]
    }
}

#[tokio::test]
async fn write_g34_with_every_subset_level_4_qualifier() {
    let variations = [
        DeadBandObjects {
            variation: 0x01,
            first: &[0x0A, 0x00],
            second: &[0xCA, 0xFE],
            deadbands: [10.0, 65226.0],
        },
        DeadBandObjects {
            variation: 0x02,
            first: &[0x70, 0x11, 0x01, 0x00],
            second: &[0x01, 0x02, 0x03, 0x04],
            deadbands: [70000.0, 67305985.0],
        },
        DeadBandObjects {
            variation: 0x03,
            first: &[0x00, 0x00, 0x20, 0x40],
            second: &[0x00, 0x00, 0xA0, 0x3F],
            deadbands: [2.5, 1.25],
        },
    ];

    for objects in variations {
        for request in objects.requests() {
            let mut harness = new_harness(get_default_config());
            harness.handle.database.transaction(|db| {
                db.add(3, Some(EventClass::Class1), AnalogInputConfig::default());
                db.add(4, Some(EventClass::Class1), AnalogInputConfig::default());
            });

            harness
                .test_request_response(&request, &empty_response(0, 0))
                .await;

            harness.check_events(&[
                Event::BeginWriteDeadBands,
                Event::WriteDeadBand(3, objects.deadbands[0]),
                Event::WriteDeadBand(4, objects.deadbands[1]),
                Event::EndWriteDeadBands,
            ]);
            harness.check_no_events();

            // the database holds the deadbands written, so a READ of the variation returns the
            // objects written
            let mut response = vec![
                0xC1,
                0x81,
                0x80,
                0x00,
                0x22,
                objects.variation,
                0x01,
                0x03,
                0x00,
                0x04,
                0x00,
            ];
            response.extend_from_slice(objects.first);
            response.extend_from_slice(objects.second);
            harness
                .test_request_response(&[0xC1, 0x01, 0x22, objects.variation, 0x06], &response)
                .await;
        }
    }
}

#[tokio::test]
async fn ranged_write_of_g34_applies_the_valid_deadbands_around_rejected_ones() {
    // g34v3 for indices 3 to 6: 1.5, then 2.0 for index 4 which isn't installed, NaN, then 2.5
    let objects: &[u8] = &[
        0x00, 0x00, 0xC0, 0x3F, 0x00, 0x00, 0x00, 0x40, 0x00, 0x00, 0xC0, 0x7F, 0x00, 0x00, 0x20,
        0x40,
    ];
    let ranges: [&[u8]; 2] = [&[0x00, 0x03, 0x06], &[0x01, 0x03, 0x00, 0x06, 0x00]];

    for range in ranges {
        let mut harness = new_harness(get_default_config());
        let config = AnalogInputConfig {
            deadband: 9.0,
            ..Default::default()
        };
        harness.handle.database.transaction(|db| {
            for index in [3, 5, 6] {
                db.add(index, Some(EventClass::Class1), config);
            }
        });

        let mut request = vec![0xC0, 0x02, 0x22, 0x03];
        request.extend_from_slice(range);
        request.extend_from_slice(objects);
        // IIN2.2 (PARAMETER_ERROR)
        harness
            .test_request_response(&request, &[0xC0, 0x81, 0x80, 0x04])
            .await;

        harness.check_events(&[
            Event::BeginWriteDeadBands,
            Event::WriteDeadBand(3, 1.5),
            Event::WriteDeadBand(6, 2.5),
            Event::EndWriteDeadBands,
        ]);
        harness.check_no_events();

        // index 3 and 6 hold the deadbands written, and index 5 keeps its deadband of 9.0
        harness
            .test_request_response(
                &[0xC1, 0x01, 0x22, 0x03, 0x06],
                &[
                    0xC1, 0x81, 0x80, 0x00, 0x22, 0x03, 0x01, 0x03, 0x00, 0x03, 0x00, 0x00, 0x00,
                    0xC0, 0x3F, 0x22, 0x03, 0x01, 0x05, 0x00, 0x06, 0x00, 0x00, 0x00, 0x10, 0x41,
                    0x00, 0x00, 0x20, 0x40,
                ],
            )
            .await;
    }
}

#[tokio::test]
async fn ranged_write_of_g34_stores_negative_deadbands_as_zero_and_rejects_nan() {
    // g34v3 for indices 3 to 6: -1.0, NaN, the smallest positive subnormal, then -infinity
    let objects: &[u8] = &[
        0x00, 0x00, 0x80, 0xBF, 0x00, 0x00, 0xC0, 0x7F, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80,
        0xFF,
    ];
    let ranges: [&[u8]; 2] = [&[0x00, 0x03, 0x06], &[0x01, 0x03, 0x00, 0x06, 0x00]];
    let subnormal = f64::from(f32::from_bits(1));

    for range in ranges {
        let mut harness = new_harness(get_default_config());
        let config = AnalogInputConfig {
            deadband: 9.0,
            ..Default::default()
        };
        harness.handle.database.transaction(|db| {
            for index in 3..=6 {
                db.add(index, Some(EventClass::Class1), config);
            }
        });

        let mut request = vec![0xC0, 0x02, 0x22, 0x03];
        request.extend_from_slice(range);
        request.extend_from_slice(objects);
        // IIN2.2 (PARAMETER_ERROR) for the NaN
        harness
            .test_request_response(&request, &[0xC0, 0x81, 0x80, 0x04])
            .await;

        harness.check_events(&[
            Event::BeginWriteDeadBands,
            Event::WriteDeadBand(3, 0.0),
            Event::WriteDeadBand(5, subnormal),
            Event::WriteDeadBand(6, 0.0),
            Event::EndWriteDeadBands,
        ]);
        harness.check_no_events();

        // 0, the unchanged 9.0, the subnormal, and 0
        harness
            .test_request_response(
                &[0xC1, 0x01, 0x22, 0x03, 0x06],
                &[
                    0xC1, 0x81, 0x80, 0x00, 0x22, 0x03, 0x01, 0x03, 0x00, 0x06, 0x00, 0x00, 0x00,
                    0x00, 0x00, 0x00, 0x00, 0x10, 0x41, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                    0x00,
                ],
            )
            .await;
    }
}
