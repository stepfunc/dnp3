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

// read g0v255 (list of attribute variations) for all sets with qualifier 0x06
const READ_ALL_ATTRIBUTE_LISTS: &[u8] = &[0xC0, 0x01, 0x00, 0xFF, 0x06];

/// define `count` unsigned integer attributes, variations 1..=count, in `set`
fn define_uint_attrs(db: &mut crate::outstation::database::Database, set: u8, count: u8) {
    for var in 1..=count {
        db.define_attr(
            AttrProp::default(),
            OwnedAttribute::new(
                AttrSet::new(set),
                var,
                OwnedAttrValue::UnsignedInt(var.into()),
            ),
        )
        .unwrap();
    }
}

/// expected encoding of the variation list for a set of unsigned integer attributes
fn encode_uint_attr_list(set: u8, count: u8) -> Vec<u8> {
    let mut bytes = vec![0x00, 0xFF, 0x00, set, set];
    let len = usize::from(count) * 2;
    match u8::try_from(len) {
        Ok(len) => bytes.extend([254, len]),
        Err(_) => bytes.extend([255, u8::try_from(len.checked_sub(256).unwrap()).unwrap()]),
    }
    for var in 1..=count {
        bytes.extend([var, 0x00]);
    }
    bytes
}

#[tokio::test]
async fn reads_list_of_attribute_variations() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::writable(),
            StringAttr::UserAssignedLocation.with_value("Bend"),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            StringAttr::ConfigVersion.with_value("1.0.0"),
        )
        .unwrap();
    });

    let response = [
        0xC0,
        0x81,
        0x80,
        0x00,
        // g0v255, 1-byte start/stop, set 0
        0x00,
        0xFF,
        0x00,
        0x00,
        0x00,
        // attribute list, 4 bytes
        254,
        4,
        // variations in ascending order with their properties
        var::CONFIG_VERSION,
        0x00,
        var::USER_ASSIGNED_LOCATION,
        0x01,
    ];

    harness
        .test_request_response(READ_ALL_ATTRIBUTE_LISTS, &response)
        .await;
}

#[tokio::test]
async fn reads_extended_list_of_attribute_variations() {
    let mut harness = new_harness(get_default_config());

    // 200 variations take 400 bytes, which needs the extended list encoding
    harness
        .handle
        .database
        .transaction(|db| define_uint_attrs(db, 1, 200));

    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(encode_uint_attr_list(1, 200));

    harness
        .test_request_response(READ_ALL_ATTRIBUTE_LISTS, &response)
        .await;
}

#[tokio::test]
async fn reads_maximum_list_of_attribute_variations() {
    let mut harness = new_harness(get_default_config());

    // every non-reserved variation in a set
    harness
        .handle
        .database
        .transaction(|db| define_uint_attrs(db, 1, 253));

    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(encode_uint_attr_list(1, 253));

    harness
        .test_request_response(READ_ALL_ATTRIBUTE_LISTS, &response)
        .await;
}

#[tokio::test]
async fn attribute_list_that_does_not_fit_is_moved_whole_to_the_next_fragment() {
    let mut config = get_default_config();
    config.solicited_buffer_size = BufferSize::min();
    let mut harness = new_harness(config);

    // each list fits in a fragment by itself, but not both together
    harness.handle.database.transaction(|db| {
        define_uint_attrs(db, 1, 60);
        define_uint_attrs(db, 2, 60);
    });

    harness.send_and_process(READ_ALL_ATTRIBUTE_LISTS).await;
    // FIR, CON, seq 0
    let mut first = vec![0xA0, 0x81, 0x80, 0x00];
    first.extend(encode_uint_attr_list(1, 60));
    harness.expect_response(&first).await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);

    harness.send_and_process(CONFIRM_SEQ_0).await;
    // FIN, seq 1
    let mut second = vec![0x41, 0x81, 0x80, 0x00];
    second.extend(encode_uint_attr_list(2, 60));
    harness.expect_response(&second).await;
}
