use crate::outstation::database::{Add, AnalogInputConfig, EventClass};
use crate::outstation::tests::harness::*;

fn config(deadband: f64) -> AnalogInputConfig {
    AnalogInputConfig {
        deadband,
        ..Default::default()
    }
}

#[tokio::test]
async fn read_g34_v1() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(42.0)));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x01, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x01, 0x01, 0x07, 0x00, 0x07, 0x00, 0x2A, 0x00,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v1_by_range() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(42.0)));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x01, 0x00, 0x07, 0x07],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x01, 0x01, 0x07, 0x00, 0x07, 0x00, 0x2A, 0x00,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v1_no_objects() {
    let mut harness = new_harness(get_default_config());

    // no analog inputs, so IIN2.1 (OBJECT_UNKNOWN)
    harness
        .test_request_response(&[0xC0, 0x01, 0x22, 0x01, 0x06], &[0xC0, 0x81, 0x80, 0x02])
        .await;
}

#[tokio::test]
async fn read_g34_v1_by_range_past_the_last_point() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(42.0)));

    // 7 to 8, so IIN2.2 (PARAMETER_ERROR) with the value for 7
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x01, 0x00, 0x07, 0x08],
            &[
                0xC0, 0x81, 0x80, 0x04, 0x22, 0x01, 0x01, 0x07, 0x00, 0x07, 0x00, 0x2A, 0x00,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v1_by_range_without_installed_points() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(42.0)));

    // 8 to 9, so IIN2.2 (PARAMETER_ERROR) with no values
    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x01, 0x00, 0x08, 0x09],
            &[0xC0, 0x81, 0x80, 0x04],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v1_overflow() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(1000000.0)));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x01, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x01, 0x01, 0x07, 0x00, 0x07, 0x00, 0xFF, 0xFF,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v2() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(42.0)));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x02, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x02, 0x01, 0x07, 0x00, 0x07, 0x00, 0x2A, 0x00, 0x00,
                0x00,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v2_overflow() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), config(5_000_000_000.0)));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x02, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x02, 0x01, 0x07, 0x00, 0x07, 0x00, 0xFF, 0xFF, 0xFF,
                0xFF,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v3() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), AnalogInputConfig::default()));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x03, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x03, 0x01, 0x07, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
                0x00,
            ],
        )
        .await;
}

#[tokio::test]
async fn read_g34_v0() {
    let mut harness = new_harness(get_default_config());

    harness
        .handle
        .database
        .transaction(|db| db.add(7, Some(EventClass::Class1), AnalogInputConfig::default()));

    harness
        .test_request_response(
            &[0xC0, 0x01, 0x22, 0x00, 0x06],
            &[
                0xC0, 0x81, 0x80, 0x00, 0x22, 0x03, 0x01, 0x07, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
                0x00,
            ],
        )
        .await;
}
