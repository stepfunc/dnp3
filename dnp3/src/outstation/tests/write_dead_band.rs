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
                .test_request_response(&request, EMPTY_RESPONSE)
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
