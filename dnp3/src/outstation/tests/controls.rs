use crate::app::parse::DecodeLogLevel;
use crate::app::variations::Group41Var2;
use crate::entry::EndpointAddress;
use crate::outstation::task::OutstationConfig;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::OperateType;
use crate::outstation::SelfAddressSupport;
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
    )
}

#[test]
fn performs_direct_operate() {
    let mut harness = new_harness(get_config());

    harness.test_request_response(
        // direct operate, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC0, 0x05, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 0, restart IIN + echo of request headers
        &[
            0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ],
    );

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
fn performs_select_before_operate() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------

    harness.test_request_response(
        // select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 0, restart IIN + echo of request headers
        &[
            0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ],
    );

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(Group41Var2::new(513), 7)),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness.test_request_response(
        // operate, seq == 1, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 1, restart IIN + echo of request headers
        &[
            0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ],
    );

    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(
            Control::G41V2(Group41Var2::new(513), 7),
            OperateType::SelectBeforeOperate,
        ),
        Event::EndControls,
    ]);
}

#[test]
fn rejects_operate_with_non_consecutive_sequence() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------

    harness.test_request_response(
        // select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 0, restart IIN + echo of request headers
        &[
            0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ],
    );

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(Group41Var2::new(513), 7)),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness.test_request_response(
        // operate, seq == 2, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC2, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 2, restart IIN + echo of request headers, but status == 02 (NO_SELECT)
        &[
            0xC2, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x02,
        ],
    );

    harness.check_no_events();
}

#[test]
fn rejects_operate_with_non_matching_headers() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------

    harness.test_request_response(
        // select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 0, restart IIN + echo of request headers
        &[
            0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ],
    );

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(Group41Var2::new(513), 7)),
        Event::EndControls,
    ]);

    // ------------ operate -------------

    harness.test_request_response(
        // use a different index (8 != 7 from OPERATE above)
        // operate, seq == 1, g41v2 - count == 1, index == 8, value = 51, status == SUCCESS
        &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x08, 0x01, 0x02, 0x00],
        // response, seq == 1, restart IIN + echo of request headers, status == 2 (NO_SELECT)
        &[
            0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x08, 0x01, 0x02, 0x02,
        ],
    );

    harness.check_no_events();
}

#[test]
fn select_can_time_out() {
    let mut harness = new_harness(get_config());

    // ------------ select -------------

    harness.test_request_response(
        // select, seq == 0, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 0, restart IIN + echo of request headers
        &[
            0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x00,
        ],
    );

    harness.check_events(&[
        Event::BeginControls,
        Event::Select(Control::G41V2(Group41Var2::new(513), 7)),
        Event::EndControls,
    ]);

    crate::tokio::time::advance(Duration::from_millis(5001));

    // ------------ operate -------------

    harness.test_request_response(
        // operate, seq == 1, g41v2 - count == 1, index == 7, value = 513, status == SUCCESS
        &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00],
        // response, seq == 1, restart IIN + echo of request headers but with STATUS == 1 (TIMEOUT)
        &[
            0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x1, 0x07, 0x01, 0x02, 0x01,
        ],
    );

    harness.check_no_events();
}
