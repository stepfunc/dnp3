use crate::app::attr::*;
use crate::app::BufferSize;
use crate::outstation::database::AttrDefError;
use crate::outstation::tests::harness::*;
use crate::util::test_support::watchdog;

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

/// an attribute with a 1-byte start/stop of `set`: its group, variation, qualifier and range
fn attr_header(set: u8, variation: u8) -> [u8; 5] {
    [0x00, variation, 0x00, set, set]
}

/// the data type code, length and value of an attribute
fn attr_value(data_type: u8, value: &[u8]) -> Vec<u8> {
    let mut bytes = vec![data_type, value.len().try_into().unwrap()];
    bytes.extend_from_slice(value);
    bytes
}

/// WRITE of one attribute
fn write_attr(seq: u8, set: u8, variation: u8, data_type: u8, value: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xC0 | seq, 0x02];
    bytes.extend(attr_header(set, variation));
    bytes.extend(attr_value(data_type, value));
    bytes
}

/// READ of one attribute
fn read_attr(seq: u8, set: u8, variation: u8) -> Vec<u8> {
    let mut bytes = vec![0xC0 | seq, 0x01];
    bytes.extend(attr_header(set, variation));
    bytes
}

/// the response to a READ of one attribute, with the restart IIN
fn attr_response(seq: u8, set: u8, variation: u8, data_type: u8, value: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xC0 | seq, 0x81, 0x80, 0x00];
    bytes.extend(attr_header(set, variation));
    bytes.extend(attr_value(data_type, value));
    bytes
}

/// an empty response with the restart IIN and `iin2`
fn empty_response(seq: u8, iin2: u8) -> [u8; 4] {
    [0xC0 | seq, 0x81, 0x80, iin2]
}

// attribute data type codes
const VSTR: u8 = 1;
const UINT: u8 = 2;
const INT: u8 = 3;
const FLT: u8 = 4;
const OSTR: u8 = 5;
const BSTR: u8 = 6;
const TIME: u8 = 7;

const IIN2_NONE: u8 = 0x00;
const IIN2_OBJECT_UNKNOWN: u8 = 0x02;
const IIN2_PARAMETER_ERROR: u8 = 0x04;

/// the default set holds a writable location "old" and a read-only config version "1.0.0"
fn define_default_set(harness: &mut OutstationHarness) {
    harness.handle.database.transaction(|db| {
        db.define_attr(
            AttrProp::writable(),
            StringAttr::UserAssignedLocation.with_value("old"),
        )
        .unwrap();
        db.define_attr(
            AttrProp::default(),
            StringAttr::ConfigVersion.with_value("1.0.0"),
        )
        .unwrap();
    });
}

/// READ the location and the config version, expecting the values they were defined with
async fn expect_default_set_unchanged(harness: &mut OutstationHarness, seq: u8) {
    harness
        .test_request_response(
            &read_attr(seq, 0, var::USER_ASSIGNED_LOCATION),
            &attr_response(seq, 0, var::USER_ASSIGNED_LOCATION, VSTR, b"old"),
        )
        .await;
    let seq = seq.wrapping_add(1);
    harness
        .test_request_response(
            &read_attr(seq, 0, var::CONFIG_VERSION),
            &attr_response(seq, 0, var::CONFIG_VERSION, VSTR, b"1.0.0"),
        )
        .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_attribute_that_is_not_writable_sets_object_unknown_and_keeps_the_value() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        define_default_set(&mut harness);

        harness
            .test_request_response(
                &write_attr(0, 0, var::CONFIG_VERSION, VSTR, b"2.0.0"),
                &empty_response(0, IIN2_OBJECT_UNKNOWN),
            )
            .await;
        harness.check_no_events();

        expect_default_set_unchanged(&mut harness, 1).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_attribute_in_undefined_set_sets_parameter_error() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        define_default_set(&mut harness);

        harness
            .test_request_response(
                &write_attr(0, 5, 1, VSTR, b"new"),
                &empty_response(0, IIN2_PARAMETER_ERROR),
            )
            .await;
        harness.check_no_events();

        expect_default_set_unchanged(&mut harness, 1).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_undefined_attribute_in_defined_set_sets_object_unknown() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        define_default_set(&mut harness);

        harness
            .test_request_response(
                &write_attr(0, 0, var::USER_ASSIGNED_ID, VSTR, b"new"),
                &empty_response(0, IIN2_OBJECT_UNKNOWN),
            )
            .await;
        harness.check_no_events();

        expect_default_set_unchanged(&mut harness, 1).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_attribute_with_the_wrong_type_sets_parameter_error_and_keeps_the_value() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        define_default_set(&mut harness);

        // the location is a visible string
        harness
            .test_request_response(
                &write_attr(0, 0, var::USER_ASSIGNED_LOCATION, UINT, &[7]),
                &empty_response(0, IIN2_PARAMETER_ERROR),
            )
            .await;
        harness.check_no_events();

        expect_default_set_unchanged(&mut harness, 1).await;
    })
    .await;
}

/// WRITE "new" to the location with the application answering as configured, expecting one
/// validation callback
async fn write_location_with_application_answer(accept: bool, defer: bool) -> OutstationHarness {
    let mut harness = new_harness(get_default_config());
    define_default_set(&mut harness);
    {
        let mut data = harness.application_data.lock().unwrap();
        data.accept_attr_writes = accept;
        data.defer_attr_writes = defer;
    }

    let iin2 = if accept {
        IIN2_NONE
    } else {
        IIN2_PARAMETER_ERROR
    };
    harness
        .test_request_response(
            &write_attr(0, 0, var::USER_ASSIGNED_LOCATION, VSTR, b"new"),
            &empty_response(0, iin2),
        )
        .await;
    harness.check_events(&[Event::WriteDeviceAttr(
        AttrSet::Default,
        var::USER_ASSIGNED_LOCATION,
    )]);
    harness.check_no_events();
    harness
}

#[tokio::test(start_paused = true)]
async fn write_of_attribute_the_application_rejects_sets_parameter_error_and_keeps_the_value() {
    watchdog(async {
        let mut harness = write_location_with_application_answer(false, false).await;
        expect_default_set_unchanged(&mut harness, 1).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_attribute_the_application_rejects_later_sets_parameter_error_and_keeps_the_value()
{
    watchdog(async {
        let mut harness = write_location_with_application_answer(false, true).await;
        expect_default_set_unchanged(&mut harness, 1).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn write_of_attribute_the_application_accepts_later_stores_the_value() {
    watchdog(async {
        let mut harness = write_location_with_application_answer(true, true).await;
        harness
            .test_request_response(
                &read_attr(1, 0, var::USER_ASSIGNED_LOCATION),
                &attr_response(1, 0, var::USER_ASSIGNED_LOCATION, VSTR, b"new"),
            )
            .await;
    })
    .await;
}

/// The writable attributes of private set 1, one of each type
fn typed_attrs() -> [(u8, OwnedAttrValue); 7] {
    [
        (1, OwnedAttrValue::UnsignedInt(0)),
        (2, OwnedAttrValue::SignedInt(0)),
        (3, OwnedAttrValue::FloatingPoint(FloatType::F64(0.0))),
        (4, OwnedAttrValue::Dnp3Time(crate::app::Timestamp::new(0))),
        (5, OwnedAttrValue::OctetString(vec![0x00])),
        (6, OwnedAttrValue::BitString(vec![0x00])),
        (7, OwnedAttrValue::VisibleString("old".to_string())),
    ]
}

#[tokio::test(start_paused = true)]
async fn writes_of_every_attribute_type_are_stored_and_read_back_in_the_smallest_encoding() {
    watchdog(async {
        let mut harness = new_harness(get_default_config());
        harness.handle.database.transaction(|db| {
            for (variation, value) in typed_attrs() {
                db.define_attr(
                    AttrProp::writable(),
                    OwnedAttribute::new(AttrSet::new(1), variation, value),
                )
                .unwrap();
            }
        });

        // (variation, data type, written bytes, bytes read back)
        let cases: &[(u8, u8, &[u8], &[u8])] = &[
            // unsigned integers at the width boundaries: 255, 256, 65535, 65536 and u32::MAX
            (1, UINT, &[0xFF], &[0xFF]),
            (1, UINT, &[0x00, 0x01], &[0x00, 0x01]),
            (1, UINT, &[0xFF, 0xFF], &[0xFF, 0xFF]),
            (
                1,
                UINT,
                &[0x00, 0x00, 0x01, 0x00],
                &[0x00, 0x00, 0x01, 0x00],
            ),
            (
                1,
                UINT,
                &[0xFF, 0xFF, 0xFF, 0xFF],
                &[0xFF, 0xFF, 0xFF, 0xFF],
            ),
            // values written wider than they need are read back in the smallest width
            (1, UINT, &[0x01, 0x00, 0x00, 0x00], &[0x01]),
            (1, UINT, &[0xFF, 0x00], &[0xFF]),
            // signed integers at the width boundaries: 127, 128, -128, -129, 32767, 32768,
            // -32768, -32769 and i32::MIN
            (2, INT, &[0x7F], &[0x7F]),
            (2, INT, &[0x80, 0x00], &[0x80, 0x00]),
            (2, INT, &[0x80], &[0x80]),
            (2, INT, &[0x7F, 0xFF], &[0x7F, 0xFF]),
            (2, INT, &[0xFF, 0x7F], &[0xFF, 0x7F]),
            (2, INT, &[0x00, 0x80, 0x00, 0x00], &[0x00, 0x80, 0x00, 0x00]),
            (2, INT, &[0x00, 0x80], &[0x00, 0x80]),
            (2, INT, &[0xFF, 0x7F, 0xFF, 0xFF], &[0xFF, 0x7F, 0xFF, 0xFF]),
            (2, INT, &[0x00, 0x00, 0x00, 0x80], &[0x00, 0x00, 0x00, 0x80]),
            // -1 written in 4 bytes is read back in 1
            (2, INT, &[0xFF, 0xFF, 0xFF, 0xFF], &[0xFF]),
            // a single-precision float keeps its width, then a double-precision one
            (3, FLT, &1.5f32.to_le_bytes(), &1.5f32.to_le_bytes()),
            (3, FLT, &(-0.1f64).to_le_bytes(), &(-0.1f64).to_le_bytes()),
            // 48-bit times, up to the largest
            (
                4,
                TIME,
                &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
                &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06],
            ),
            (4, TIME, &[0xFF; 6], &[0xFF; 6]),
            (5, OSTR, &[0xDE, 0xAD, 0xBE], &[0xDE, 0xAD, 0xBE]),
            (6, BSTR, &[0xAA, 0x55], &[0xAA, 0x55]),
            (7, VSTR, b"new", b"new"),
        ];

        let mut seq: u8 = 0;
        for &(variation, data_type, written, read_back) in cases {
            harness
                .test_request_response(
                    &write_attr(seq, 1, variation, data_type, written),
                    &empty_response(seq, IIN2_NONE),
                )
                .await;
            harness.check_events(&[Event::WriteDeviceAttr(AttrSet::Private(1), variation)]);
            seq = seq.wrapping_add(1) & 0x0F;

            harness
                .test_request_response(
                    &read_attr(seq, 1, variation),
                    &attr_response(seq, 1, variation, data_type, read_back),
                )
                .await;
            seq = seq.wrapping_add(1) & 0x0F;
        }
        harness.check_no_events();
    })
    .await;
}
