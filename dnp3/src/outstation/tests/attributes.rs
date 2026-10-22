use crate::app::attr::*;
use crate::app::BufferSize;
use crate::outstation::database::AttrDefError;
use crate::outstation::tests::harness::*;

// read g0v254 (all attributes) with qualifier 0x06
const READ_ALL_ATTRIBUTES: &[u8] = &[0xC0, 0x01, 0x00, 0xFE, 0x06];

#[tokio::test]
async fn attribute_values_longer_than_255_bytes_cannot_be_defined() {
    let harness = new_harness(get_default_config());

    let results = harness.handle.database.transaction(|db| {
        [
            db.define_attr(
                AttrProp::default(),
                StringAttr::DeviceSerialNumber.with_value("L".repeat(256)),
            ),
            db.define_attr(
                AttrProp::default(),
                OwnedAttribute::new(
                    AttrSet::new(1),
                    1,
                    OwnedAttrValue::OctetString(vec![0xAA; 256]),
                ),
            ),
            db.define_attr(
                AttrProp::default(),
                OwnedAttribute::new(
                    AttrSet::new(1),
                    2,
                    OwnedAttrValue::BitString(vec![0xBB; 256]),
                ),
            ),
        ]
    });

    assert_eq!(
        results,
        [Err(AttrDefError::BadValue(BadAttribute::BadLength(256))); 3]
    );
}

#[tokio::test]
async fn attribute_values_of_255_bytes_are_defined_and_read_back() {
    let mut harness = new_harness(get_default_config());

    let serial = "S".repeat(255);

    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::default(),
            StringAttr::DeviceSerialNumber.with_value(serial.as_str()),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            OwnedAttribute::new(
                AttrSet::new(1),
                1,
                OwnedAttrValue::OctetString(vec![0xAA; 255]),
            ),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            OwnedAttribute::new(
                AttrSet::new(1),
                2,
                OwnedAttrValue::BitString(vec![0xBB; 255]),
            ),
        )
        .unwrap();
    });

    let mut response = vec![0xC0, 0x81, 0x80, 0x00];
    response.extend(encode_string(var::DEVICE_SERIAL_NUMBER, &serial));
    // g0v1, set 1, octet string, 255 bytes
    response.extend([0x00, 0x01, 0x00, 0x01, 0x01, 0x05, 0xFF]);
    response.extend([0xAA; 255]);
    // g0v2, set 1, bit string, 255 bytes
    response.extend([0x00, 0x02, 0x00, 0x01, 0x01, 0x06, 0xFF]);
    response.extend([0xBB; 255]);

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

    harness.send_and_process(&confirm(0)).await;
    // FIN, seq 1
    let mut second = vec![0x41, 0x81, 0x80, 0x00];
    second.extend(encode_uint_attr_list(2, 60));
    harness.expect_response(&second).await;
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

#[tokio::test]
async fn writes_attribute_with_one_and_two_byte_ranges() {
    let mut harness = new_harness(get_default_config());

    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::writable(),
            StringAttr::UserAssignedLocation.with_value("old"),
        )
        .unwrap();
    });

    // read g0v245 in the default set with a 1-byte start/stop
    const READ_LOCATION: [u8; 6] = [0x01, 0x00, var::USER_ASSIGNED_LOCATION, 0x00, 0x00, 0x00];

    // write g0v245 in the default set, 1-byte start/stop then 2-byte start/stop
    let writes: [&[u8]; 2] = [
        &[
            0xC0,
            0x02,
            0x00,
            var::USER_ASSIGNED_LOCATION,
            0x00,
            0x00,
            0x00,
            0x01,
            3,
            b'o',
            b'n',
            b'e',
        ],
        &[
            0xC2,
            0x02,
            0x00,
            var::USER_ASSIGNED_LOCATION,
            0x01,
            0x00,
            0x00,
            0x00,
            0x00,
            0x01,
            3,
            b't',
            b'w',
            b'o',
        ],
    ];

    for (seq, (write, value)) in writes.into_iter().zip(["one", "two"]).enumerate() {
        let seq: u8 = (2 * seq).try_into().unwrap();
        harness
            .test_request_response(write, &[0xC0 | seq, 0x81, 0x80, 0x00])
            .await;

        let mut read = vec![0xC0 | (seq + 1)];
        read.extend(READ_LOCATION);
        let mut response = vec![0xC0 | (seq + 1), 0x81, 0x80, 0x00];
        response.extend(encode_string(var::USER_ASSIGNED_LOCATION, value));
        harness.test_request_response(&read, &response).await;
    }
}
