use crate::app::measurement::OctetString;
use crate::outstation::database::EventClass::{Class1, Class2};
use crate::outstation::database::{Add, OctetStringConfig, Update, UpdateOptions};
use crate::outstation::tests::harness::*;

#[tokio::test]
async fn objects_of_same_length_are_encoded_in_a_single_header() {
    let mut harness = new_harness(get_default_config());

    harness.handle.transaction(|db| {
        db.add(0, Some(Class1), OctetStringConfig);
        db.add(1, Some(Class1), OctetStringConfig);
        db.add(2, Some(Class1), OctetStringConfig);
    });

    // respond with a single header with 3 group 110 var 1, indices 0 to 2 all containing value [0x00]
    let response = [0xC0, 0x81, 0x80, 0x00, 110, 1, 1, 0, 0, 2, 0, 0, 0, 0];

    harness
        .test_request_response(&[0xC0, 0x01, 110, 0, 0x06], &response)
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn objects_of_different_lengths_are_encoded_in_individual_headers() {
    let mut harness = new_harness(get_default_config());

    let value0 = OctetString::new(&[0xAA]).unwrap();
    let value1 = OctetString::new(&[0xAA, 0xBB]).unwrap();
    let value2 = OctetString::new(&[0xAA, 0xBB, 0xCC]).unwrap();

    harness.handle.transaction(|db| {
        db.add(0, None, OctetStringConfig);
        db.add(1, None, OctetStringConfig);
        db.add(2, None, OctetStringConfig);

        db.update(0, &value0, UpdateOptions::default());
        db.update(1, &value1, UpdateOptions::default());
        db.update(2, &value2, UpdateOptions::default());
    });

    // respond with 3 different headers
    let response = [
        0xC0, 0x81, 0x80, 0x00, // header #1
        110, 1, 1, 0, 0, 0, 0, 0xAA, // header #2
        110, 2, 1, 1, 0, 1, 0, 0xAA, 0xBB, // header #3
        110, 3, 1, 2, 0, 2, 0, 0xAA, 0xBB, 0xCC,
    ];

    harness
        .test_request_response(&[0xC0, 0x01, 110, 0, 0x06], &response)
        .await;

    harness.check_no_events();
}

#[tokio::test]
async fn events_with_same_length_are_encoded_in_the_same_header() {
    let mut harness = new_harness(get_default_config());

    let value0 = OctetString::new(&[0xAA]).unwrap();
    let value1 = OctetString::new(&[0xBB]).unwrap();

    harness.handle.transaction(|db| {
        db.add(0, Some(Class2), OctetStringConfig);
        db.add(1, Some(Class2), OctetStringConfig);

        db.update(0, &value0, UpdateOptions::default());
        db.update(1, &value1, UpdateOptions::default());
    });

    // respond with 3 different headers requesting confirmation
    let response = [
        0xE0, 0x81, 0x80, 0x00, // single header w/ two values
        111, 1, 0x28, 2, 0, 0, 0, 0xAA, 1, 0, 0xBB,
    ];

    harness
        // read class 2 events
        .test_request_response(&[0xC0, 0x01, 60, 3, 0x06], &response)
        .await;

    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
}

#[tokio::test]
async fn events_with_different_lengths_use_different_headers() {
    let mut harness = new_harness(get_default_config());

    let value0 = OctetString::new(&[0xAA]).unwrap();
    let value1 = OctetString::new(&[0xAA, 0xBB]).unwrap();

    harness.handle.transaction(|db| {
        db.add(0, Some(Class2), OctetStringConfig);
        db.add(1, Some(Class2), OctetStringConfig);

        db.update(0, &value0, UpdateOptions::default());
        db.update(1, &value1, UpdateOptions::default());
    });

    // respond with 3 different headers requesting confirmation
    let response = [
        0xE0, 0x81, 0x80, 0x00, 111, 1, 0x28, 1, 0, 0, 0, 0xAA, // header #1
        111, 2, 0x28, 1, 0, 1, 0, 0xAA, 0xBB, // header #2
    ];

    harness
        // read class 2 events
        .test_request_response(&[0xC0, 0x01, 60, 3, 0x06], &response)
        .await;

    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
}
