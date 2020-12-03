use crate::app::parse::DecodeLogLevel;
use crate::app::variations::Group41Var2;
use crate::entry::EndpointAddress;
use crate::outstation::task::OutstationConfig;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::OperateType;
use crate::outstation::{BroadcastAddressSupport, SelfAddressSupport};
use tokio::time::Duration;

fn get_config() -> OutstationConfig {
    OutstationConfig::new(
        2048,
        2048,
        EndpointAddress::from(10).unwrap(),
        EndpointAddress::from(1).unwrap(),
        SelfAddressSupport::Disabled,
        DecodeLogLevel::ObjectValues,
        Duration::from_secs(2),
        Duration::from_secs(5),
        BroadcastAddressSupport::Enabled,
    )
}

const G41V2_VALUE: Group41Var2 = Group41Var2::new(513);
// select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS,
const SELECT_SEQ0_G41V2: &[u8] = &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// operate, seq == 1, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const OPERATE_SEQ1_G41V2: &[u8] = &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// operate, seq == 1, g41v2 - count == 1, index == 8, value = 513, status == SUCCESS
const OPERATE_SEQ1_G41V2_INDEX_8: &[u8] = &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x08, 0x01, 0x02, 0x00];
// operate, seq == 2, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const OPERATE_SEQ2_G41V2: &[u8] = &[0xC2, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// direct operate, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const DO_SEQ0_G41V2: &[u8] = &[0xC0, 0x05, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// direct operate no ack, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
const DO_NO_ACK_SEQ0_G41V2: &[u8] = &[0xC0, 0x06, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
// response, seq == 0, restart IIN + echo of request headers
const RESPONSE_SEQ0_G41V2_SUCCESS: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
];
// response, seq == 1, restart IIN + echo of request headers
const RESPONSE_SEQ1_G41V2_SUCCESS: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
];
// response, seq == 2, restart IIN + echo of request headers but with status == NO_SELECT
const RESPONSE_SEQ2_G41V2_NO_SELECT: &[u8] = &[
    0xC2, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x02,
];
// response, seq == 1, restart IIN + echo of request headers, index == 8, status == 2 (NO_SELECT)
const RESPONSE_SEQ1_G41V2_INDEX8_NO_SELECT: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x08, 0x01, 0x02, 0x02,
];
// response, seq == 1, restart IIN + echo of request headers but with STATUS == 1 (TIMEOUT)
const RESPONSE_SEQ1_G41V2_SELECT_TIMEOUT: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x01,
];

#[test]
fn performs_direct_operate() {
    let mut harness = new_harness(get_config());

    harness.test_request_response(DO_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS);

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(Group41Var2::new(513), 7),
            OperateType::DirectOperate,
        ),
        Event::EndControls,
    ]);
}

#[test]
fn performs_direct_operate_no_ack() {
    let mut harness = new_harness(get_config());

    harness.test_request_no_response(DO_NO_ACK_SEQ0_G41V2);

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(G41V2_VALUE, 7),
            OperateType::DirectOperateNoAck,
        ),
        Event::EndControls,
    ]);
}

#[test]
fn performs_select_before_operate() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------

    harness.test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS);

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(G41V2_VALUE, 7)),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness.test_request_response(OPERATE_SEQ1_G41V2, RESPONSE_SEQ1_G41V2_SUCCESS);

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(G41V2_VALUE, 7),
            OperateType::SelectBeforeOperate,
        ),
        Event::EndControls,
    ]);
}

#[test]
fn rejects_operate_with_non_consecutive_sequence() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------
    harness.test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS);

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(G41V2_VALUE, 7)),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness.test_request_response(OPERATE_SEQ2_G41V2, RESPONSE_SEQ2_G41V2_NO_SELECT);

    harness.check_no_events();
}

#[test]
fn rejects_operate_with_non_matching_headers() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------
    harness.test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS);

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(G41V2_VALUE, 7)),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness.test_request_response(
        OPERATE_SEQ1_G41V2_INDEX_8,
        RESPONSE_SEQ1_G41V2_INDEX8_NO_SELECT,
    );

    harness.check_no_events();
}

#[test]
fn select_can_time_out() {
    let config = get_config();
    let mut harness = new_harness(config);

    // ------------ select -------------
    harness.test_request_response(SELECT_SEQ0_G41V2, RESPONSE_SEQ0_G41V2_SUCCESS);

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(G41V2_VALUE, 7)),
        Event::EndControls,
    ]);

    crate::tokio::time::advance(config.select_timeout + Duration::from_millis(1));

    // ------------ operate -------------

    harness.test_request_response(OPERATE_SEQ1_G41V2, RESPONSE_SEQ1_G41V2_SELECT_TIMEOUT);

    harness.check_no_events();
}
