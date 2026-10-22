#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests compute expected counts and fragment sizes"
)]

use crate::app::attr::*;
use crate::app::measurement::*;
use crate::app::BufferSize;
use crate::outstation::config::OutstationConfig;
use crate::outstation::database::*;
use crate::outstation::tests::harness::*;

const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const NULL_UNSOL_SEQ_0: &[u8] = &[0xF0, 0x82, 0x80, 0x00];
const UNS_CONFIRM_SEQ_0: &[u8] = &[0xD0, 0x00];
const ENABLE_UNSOLICITED_SEQ0: &[u8] = &[
    0xC0, 0x14, 0x3C, 0x02, 0x06, 0x3C, 0x03, 0x06, 0x3C, 0x04, 0x06,
];
const EMPTY_RESPONSE_SEQ0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const UNSOL_G2V1_SEQ1: &[u8] = &[
    0xF1, 0x82, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
];

/// g1v0, 8-bit start/stop, 0 to 0
const READ_BINARY_0: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0x00];
/// g1v1, 16-bit start/stop, 0 to 0, value true
const BINARY_0_VALUE: &[u8] = &[0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01];
/// g60v1 (class 0)
const READ_CLASS_0: &[u8] = &[0x3C, 0x01, 0x06];
/// g60v2 (class 1)
const READ_CLASS_1: &[u8] = &[0x3C, 0x02, 0x06];
/// g0v254 (all attributes), 8-bit start/stop, set 0
const READ_ALL_ATTRIBUTES_SET_0: &[u8] = &[0x00, 0xFE, 0x00, 0x00, 0x00];

const IIN2_NONE: u8 = 0x00;
const IIN2_PARAMETER_ERROR: u8 = 0x04;

fn config_with_max_read_headers(max: Option<u16>) -> OutstationConfig {
    let mut config = get_default_config();
    config.max_read_request_headers = max;
    config
}

fn read_request(header: &[u8], count: usize) -> Vec<u8> {
    let mut request = vec![0xC0, 0x01];
    for _ in 0..count {
        request.extend_from_slice(header);
    }
    request
}

fn add_binary_0(database: &mut Database) {
    database.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
    database.update(
        0,
        &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
        UpdateOptions::no_event(),
    );
}

/// the response to a READ of binary input 0 that reports it `reported` times
fn binary_0_response(reported: usize, iin2: u8) -> Vec<u8> {
    let mut response = vec![0xC0, 0x81, 0x80, iin2];
    for _ in 0..reported {
        response.extend_from_slice(BINARY_0_VALUE);
    }
    response
}

async fn test_binary_0_reads(config: OutstationConfig, count: usize, reported: usize, iin2: u8) {
    let mut harness = new_harness(config);
    harness.handle.database.transaction(add_binary_0);
    harness
        .test_request_response(
            &read_request(READ_BINARY_0, count),
            &binary_0_response(reported, iin2),
        )
        .await;
}

#[tokio::test]
async fn read_with_the_default_header_limit_has_no_error() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    test_binary_0_reads(config_with_max_read_headers(None), max, max, IIN2_NONE).await;
}

#[tokio::test]
async fn read_past_the_default_header_limit_sets_parameter_error() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    test_binary_0_reads(
        config_with_max_read_headers(None),
        max + 1,
        max,
        IIN2_PARAMETER_ERROR,
    )
    .await;
}

#[tokio::test]
async fn read_header_limit_follows_the_configured_value() {
    test_binary_0_reads(config_with_max_read_headers(Some(100)), 100, 100, IIN2_NONE).await;
    test_binary_0_reads(
        config_with_max_read_headers(Some(100)),
        101,
        100,
        IIN2_PARAMETER_ERROR,
    )
    .await;
}

#[tokio::test]
async fn read_header_limit_below_the_default_uses_the_default() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    test_binary_0_reads(config_with_max_read_headers(Some(10)), max, max, IIN2_NONE).await;
    test_binary_0_reads(
        config_with_max_read_headers(Some(10)),
        max + 1,
        max,
        IIN2_PARAMETER_ERROR,
    )
    .await;
}

async fn class_0_read_iin2(count: usize) -> u8 {
    let mut harness = new_harness(get_default_config());
    harness.handle.database.transaction(|db| {
        add_binary_0(db);
        db.add(0, Some(EventClass::Class1), CounterConfig::default());
    });
    harness
        .send_and_process(&read_request(READ_CLASS_0, count))
        .await;
    let response = harness.expect_write().await;
    assert_eq!(response.get(..3), Some([0xC0, 0x81, 0x80].as_slice()));
    response[3]
}

#[tokio::test]
async fn class_0_read_uses_a_static_selection_per_installed_type() {
    // two installed types, so 32 class 0 headers use all 64 static selections
    assert_eq!(class_0_read_iin2(32).await, IIN2_NONE);
    assert_eq!(class_0_read_iin2(33).await, IIN2_PARAMETER_ERROR);
}

#[tokio::test]
async fn read_of_event_classes_past_the_header_limit_has_no_error() {
    let mut harness = new_harness(get_default_config());
    let count = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS) + 1;
    harness
        .test_request_response(&read_request(READ_CLASS_1, count), EMPTY_RESPONSE_SEQ0)
        .await;
}

/// IIN2 of a READ of `count` copies of `header` that was deferred by an unsolicited confirm wait
async fn deferred_read_iin2(max_read_headers: Option<u16>, header: &[u8], count: usize) -> u8 {
    let mut config = get_default_unsolicited_config();
    config.max_read_request_headers = max_read_headers;
    let mut harness = new_harness(config);

    harness.expect_response(NULL_UNSOL_SEQ_0).await;
    harness.send_and_process(UNS_CONFIRM_SEQ_0).await;
    harness
        .test_request_response(ENABLE_UNSOLICITED_SEQ0, EMPTY_RESPONSE_SEQ0)
        .await;
    harness.handle.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });
    harness.expect_response(UNSOL_G2V1_SEQ1).await;

    // deferred until the unsolicited confirm wait ends
    harness.send_and_process(&read_request(header, count)).await;
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
    let response = harness.expect_write().await;
    assert_eq!(response.get(1), Some(&0x81));
    response[3]
}

#[tokio::test]
async fn deferred_read_with_the_default_header_limit_has_no_error() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    assert_eq!(
        deferred_read_iin2(None, READ_BINARY_0, max).await,
        IIN2_NONE
    );
}

#[tokio::test]
async fn deferred_read_with_the_configured_header_limit_has_no_error() {
    assert_eq!(
        deferred_read_iin2(Some(100), READ_BINARY_0, 100).await,
        IIN2_NONE
    );
}

#[tokio::test]
async fn deferred_read_past_the_configured_header_limit_sets_parameter_error() {
    assert_eq!(
        deferred_read_iin2(Some(100), READ_BINARY_0, 101).await,
        IIN2_PARAMETER_ERROR
    );
}

#[tokio::test]
async fn deferred_read_header_limit_below_the_default_uses_the_default() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    assert_eq!(
        deferred_read_iin2(Some(10), READ_BINARY_0, max).await,
        IIN2_NONE
    );
}

#[tokio::test]
async fn deferred_read_past_the_default_when_configured_below_it_sets_parameter_error() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    assert_eq!(
        deferred_read_iin2(Some(10), READ_BINARY_0, max + 1).await,
        IIN2_PARAMETER_ERROR
    );
}

#[tokio::test]
async fn deferred_read_of_event_classes_at_the_header_limit_has_no_error() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    assert_eq!(deferred_read_iin2(None, READ_CLASS_1, max).await, IIN2_NONE);
}

#[tokio::test]
async fn deferred_read_header_limit_counts_event_class_headers() {
    let max = usize::from(OutstationConfig::DEFAULT_MAX_READ_REQUEST_HEADERS);
    assert_eq!(
        deferred_read_iin2(None, READ_CLASS_1, max + 1).await,
        IIN2_PARAMETER_ERROR
    );
}

fn define_serial_number(database: &mut Database) {
    database
        .define_attr(
            AttrProp::default(),
            StringAttr::DeviceSerialNumber.with_value("S"),
        )
        .unwrap();
}

/// g0v248 (device serial number), set 0, visible string "S"
const SERIAL_NUMBER: &[u8] = &[0x00, 0xF8, 0x00, 0x00, 0x00, 0x01, 0x01, b'S'];

async fn test_attribute_reads(config: OutstationConfig, count: usize, reported: usize, iin2: u8) {
    let mut harness = new_harness(config);
    harness.handle.database.transaction(define_serial_number);
    let mut response = vec![0xC0, 0x81, 0x80, iin2];
    for _ in 0..reported {
        response.extend_from_slice(SERIAL_NUMBER);
    }
    harness
        .test_request_response(&read_request(READ_ALL_ATTRIBUTES_SET_0, count), &response)
        .await;
}

#[tokio::test]
async fn read_of_32_attribute_selections_has_no_error() {
    test_attribute_reads(get_default_config(), 32, 32, IIN2_NONE).await;
}

#[tokio::test]
async fn read_of_33_attribute_selections_sets_parameter_error() {
    test_attribute_reads(get_default_config(), 33, 32, IIN2_PARAMETER_ERROR).await;
}

#[tokio::test]
async fn attribute_selection_limit_does_not_follow_the_header_limit() {
    test_attribute_reads(
        config_with_max_read_headers(Some(100)),
        33,
        32,
        IIN2_PARAMETER_ERROR,
    )
    .await;
}

fn config_with_min_buffer() -> OutstationConfig {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    config
}

/// g1v2 values 0 to `count - 1`, 16-bit start/stop, with just RESTART set
fn g1v2_restart_values(start: u16, count: u16) -> Vec<u8> {
    let stop = start + count - 1;
    let mut objects = vec![0x01, 0x02, 0x01];
    objects.extend(start.to_le_bytes());
    objects.extend(stop.to_le_bytes());
    objects.extend(std::iter::repeat_n(0x02, count.into()));
    objects
}

fn add_binary_inputs(count: u16) -> impl FnMut(&mut Database) {
    move |db| {
        for index in 0..count {
            db.add(
                index,
                Some(EventClass::Class1),
                BinaryInputConfig::default(),
            );
        }
    }
}

/// g1v2, 16-bit start/stop
fn read_g1v2(start: u16, stop: u16) -> Vec<u8> {
    let mut request = vec![0xC0, 0x01, 0x01, 0x02, 0x01];
    request.extend(start.to_le_bytes());
    request.extend(stop.to_le_bytes());
    request
}

// a response header and a g1v2 header leave 238 bytes for values in a minimum-sized fragment
fn g1v2_values_in_min_fragment() -> u16 {
    u16::try_from(<BufferSize>::MIN - 4 - 7).unwrap()
}

#[tokio::test]
async fn static_values_that_exactly_fill_a_fragment_are_sent_in_one() {
    let count = g1v2_values_in_min_fragment();
    let mut harness = new_harness(config_with_min_buffer());
    harness
        .handle
        .database
        .transaction(add_binary_inputs(count));

    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(g1v2_restart_values(0, count));
    assert_eq!(response.len(), <BufferSize>::MIN);
    harness
        .test_request_response(&read_g1v2(0, count - 1), &response)
        .await;
}

#[tokio::test]
async fn static_value_one_byte_past_a_fragment_is_sent_in_the_next() {
    let count = g1v2_values_in_min_fragment() + 1;
    let mut harness = new_harness(config_with_min_buffer());
    harness
        .handle
        .database
        .transaction(add_binary_inputs(count));

    harness.send_and_process(&read_g1v2(0, count - 1)).await;
    // FIR, CON, seq 0
    let mut first = vec![0xA0, 0x81, 0x80, 0x00];
    first.extend(g1v2_restart_values(0, count - 1));
    harness.expect_response(&first).await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);
    // FIN, seq 1
    let mut last = vec![0x41, 0x81, 0x80, 0x00];
    last.extend(g1v2_restart_values(count - 1, 1));
    harness.expect_response(&last).await;
}

/// a visible string attribute in the default set
fn encode_string(variation: u8, value: &str) -> Vec<u8> {
    let mut bytes = vec![0x00, variation, 0x00, 0x00, 0x00, 0x01];
    bytes.push(value.len().try_into().unwrap());
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

fn define_two_strings(serial: &str, product: &str) -> impl FnMut(&mut Database) {
    let serial = serial.to_owned();
    let product = product.to_owned();
    move |db| {
        db.define_attr(
            AttrProp::default(),
            StringAttr::DeviceSerialNumber.with_value(serial.as_str()),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            StringAttr::ProductNameAndModel.with_value(product.as_str()),
        )
        .unwrap();
    }
}

// a response header and two 7-byte attribute headers leave 231 bytes for the two values
const SERIAL_LEN: usize = 115;
const PRODUCT_LEN_TO_FILL: usize = <BufferSize>::MIN - 4 - 7 - 7 - SERIAL_LEN;

#[tokio::test]
async fn attributes_that_exactly_fill_a_fragment_are_sent_in_one() {
    let serial = "S".repeat(SERIAL_LEN);
    let product = "P".repeat(PRODUCT_LEN_TO_FILL);
    let mut harness = new_harness(config_with_min_buffer());
    harness
        .handle
        .database
        .transaction(define_two_strings(&serial, &product));

    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(encode_string(var::DEVICE_SERIAL_NUMBER, &serial));
    response.extend(encode_string(var::PRODUCT_NAME_AND_MODEL, &product));
    assert_eq!(response.len(), <BufferSize>::MIN);
    harness
        .test_request_response(&[0xC0, 0x01, 0x00, 0xFE, 0x06], &response)
        .await;
}

#[tokio::test]
async fn attribute_one_byte_past_a_fragment_is_sent_in_the_next() {
    let serial = "S".repeat(SERIAL_LEN);
    let product = "P".repeat(PRODUCT_LEN_TO_FILL + 1);
    let mut harness = new_harness(config_with_min_buffer());
    harness
        .handle
        .database
        .transaction(define_two_strings(&serial, &product));

    harness
        .send_and_process(&[0xC0, 0x01, 0x00, 0xFE, 0x06])
        .await;
    // FIR, CON, seq 0
    let mut first = vec![0xA0, 0x81, 0x80, 0x00];
    first.extend(encode_string(var::DEVICE_SERIAL_NUMBER, &serial));
    harness.expect_response(&first).await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_0).await;
    harness.check_events(&[Event::SolicitedConfirmReceived(0)]);
    // FIN, seq 1
    let mut last = vec![0x41, 0x81, 0x80, 0x00];
    last.extend(encode_string(var::PRODUCT_NAME_AND_MODEL, &product));
    harness.expect_response(&last).await;
}
