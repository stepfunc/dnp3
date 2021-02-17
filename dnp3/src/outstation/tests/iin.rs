use super::harness::*;

#[test]
fn incomplete_request() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_no_response(
        &[0xC0], // Incomplete request
    );
}

#[test]
fn function_code_does_not_exist() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(
        &[0xC0, 0x70],             // Invalid function code 0x70
        &[0xC0, 0x81, 0x80, 0x01], // IIN2.0 NO_FUNC_CODE_SUPPORT set
    );
}

#[test]
fn function_code_not_supported() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(
        &[0xC0, 0x13],             // Function code SAVE_CONFIG (0x13) is not supported
        &[0xC0, 0x81, 0x80, 0x01], // IIN2.0 NO_FUNC_CODE_SUPPORT set
    );
}

#[test]
fn object_unknown() {
    let mut harness = new_harness(get_default_config());

    harness.test_request_response(
        &[0xC0, 0x01, 0x00, 0x00], // Read g0v0
        &[0xC0, 0x81, 0x80, 0x02], // IIN2.1 OBJECT_UNKNOWN set
    );
}
