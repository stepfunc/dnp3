use crate::app::attr::*;
use crate::app::BufferSize;
use crate::outstation::tests::harness::*;

// read g0v254 (all attributes) with qualifier 0x06
const READ_ALL_ATTRIBUTES: &[u8] = &[0xC0, 0x01, 0x00, 0xFE, 0x06];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];

/// expected encoding of a visible string attribute in the default set
fn encode_string(variation: u8, value: &str) -> Vec<u8> {
    let mut bytes = vec![0x00, variation, 0x00, 0x00, 0x00, 0x01];
    bytes.push(value.len().try_into().unwrap());
    bytes.extend_from_slice(value.as_bytes());
    bytes
}

#[tokio::test]
async fn attribute_that_does_not_fit_is_moved_whole_to_the_next_fragment() {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    let mut harness = new_harness(config);

    // each attribute fits in a fragment by itself, but not both together
    let serial = "S".repeat(150);
    let product = "P".repeat(150);

    harness.handle.database.transaction(|db| {
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
    });

    harness.send_and_process(READ_ALL_ATTRIBUTES).await;
    // FIR, CON, seq 0
    let mut first = vec![0xA0, 0x81, 0x80, 0x00];
    first.extend(encode_string(var::DEVICE_SERIAL_NUMBER, &serial));
    harness.expect_response(&first).await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_0).await;
    // FIN, seq 1
    let mut second = vec![0x41, 0x81, 0x80, 0x00];
    second.extend(encode_string(var::PRODUCT_NAME_AND_MODEL, &product));
    harness.expect_response(&second).await;
}

#[tokio::test]
async fn attribute_that_cannot_be_encoded_is_skipped_without_a_partial_header() {
    let mut harness = new_harness(get_default_config());

    // a visible string longer than 255 bytes cannot be encoded
    let too_long = "L".repeat(256);

    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::default(),
            StringAttr::DeviceSerialNumber.with_value("serial"),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            StringAttr::DeviceSubsetAndConformance.with_value(too_long.as_str()),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            StringAttr::ProductNameAndModel.with_value("product"),
        )
        .unwrap();
    });

    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(encode_string(var::DEVICE_SERIAL_NUMBER, "serial"));
    response.extend(encode_string(var::PRODUCT_NAME_AND_MODEL, "product"));

    harness
        .test_request_response(READ_ALL_ATTRIBUTES, &response)
        .await;
}

#[tokio::test]
async fn attribute_defined_in_private_set_zero_is_read_from_the_default_set() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::default(),
            OwnedAttribute::new(
                AttrSet::Private(0),
                var::DEVICE_SERIAL_NUMBER,
                OwnedAttrValue::VisibleString("serial".to_string()),
            ),
        )
        .unwrap();
    });

    // read g0v248 (device serial number) in set 0
    let request = &[
        0xC0,
        0x01,
        0x00,
        var::DEVICE_SERIAL_NUMBER,
        0x00,
        0x00,
        0x00,
    ];
    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(encode_string(var::DEVICE_SERIAL_NUMBER, "serial"));

    harness.test_request_response(request, &response).await;
}
