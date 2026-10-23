// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

#![expect(
    clippy::arithmetic_side_effects,
    reason = "tests count the controls a limit admits"
)]

use crate::app::control::*;
use crate::app::FunctionCode;
use crate::outstation::tests::harness::*;
use crate::outstation::traits::OperateType;

use CommandStatus::{HardwareError, NoSelect, NotSupported, Success, TooManyOps};

#[derive(Copy, Clone, Debug)]
enum Object {
    G12V1,
    G41V1,
    G41V2,
    G41V3,
    G41V4,
}

const OBJECTS: [Object; 5] = [
    Object::G12V1,
    Object::G41V1,
    Object::G41V2,
    Object::G41V3,
    Object::G41V4,
];

impl Object {
    fn group_var(self) -> [u8; 2] {
        match self {
            Object::G12V1 => [12, 1],
            Object::G41V1 => [41, 1],
            Object::G41V2 => [41, 2],
            Object::G41V3 => [41, 3],
            Object::G41V4 => [41, 4],
        }
    }

    /// the encoded object, without its trailing status byte
    fn value(self) -> &'static [u8] {
        match self {
            // CLOSE + PULSE_ON, count 1, on 100 ms, off 200 ms
            Object::G12V1 => &[0x41, 0x01, 100, 0, 0, 0, 200, 0, 0, 0],
            // -7
            Object::G41V1 => &[0xF9, 0xFF, 0xFF, 0xFF],
            // 513
            Object::G41V2 => &[0x01, 0x02],
            // 2.5
            Object::G41V3 => &[0x00, 0x00, 0x20, 0x40],
            // 2.5
            Object::G41V4 => &[0, 0, 0, 0, 0, 0, 0x04, 0x40],
        }
    }

    /// the object as the control handler receives it
    fn control(self, index: u16, status: CommandStatus) -> Control {
        match self {
            Object::G12V1 => Control::G12V1(
                Group12Var1 {
                    code: ControlCode::from_tcc_and_op_type(TripCloseCode::Close, OpType::PulseOn),
                    count: 1,
                    on_time: 100,
                    off_time: 200,
                    status,
                },
                index,
            ),
            Object::G41V1 => Control::G41V1(Group41Var1 { value: -7, status }, index),
            Object::G41V2 => Control::G41V2(Group41Var2 { value: 513, status }, index),
            Object::G41V3 => Control::G41V3(Group41Var3 { value: 2.5, status }, index),
            Object::G41V4 => Control::G41V4(Group41Var4 { value: 2.5, status }, index),
        }
    }
}

/// width of the count and index prefix
#[derive(Copy, Clone, Debug)]
enum Width {
    One,
    Two,
}

const WIDTHS: [Width; 2] = [Width::One, Width::Two];

impl Width {
    fn qualifier(self) -> u8 {
        match self {
            Width::One => 0x17,
            Width::Two => 0x28,
        }
    }

    fn write(self, value: u16, out: &mut Vec<u8>) {
        match self {
            Width::One => out.push(u8::try_from(value).unwrap()),
            Width::Two => out.extend_from_slice(&value.to_le_bytes()),
        }
    }

    /// two indices that fit the width, the two-byte ones using both bytes
    fn indices(self) -> [u16; 2] {
        match self {
            Width::One => [3, 4],
            Width::Two => [259, 513],
        }
    }
}

/// a header of one object type, and the index and status of each control in it
#[derive(Clone, Debug)]
struct Header {
    object: Object,
    width: Width,
    controls: Vec<(u16, CommandStatus)>,
}

impl Header {
    fn new(object: Object, width: Width, indices: &[u16]) -> Self {
        Self {
            object,
            width,
            controls: indices.iter().map(|&i| (i, Success)).collect(),
        }
    }

    /// two controls at `width.indices()`
    fn pair(object: Object, width: Width) -> Self {
        Self::new(object, width, &width.indices())
    }

    fn with_status(&self, mut status: impl FnMut(u16) -> CommandStatus) -> Self {
        Self {
            controls: self.controls.iter().map(|&(i, _)| (i, status(i))).collect(),
            ..self.clone()
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.object.group_var());
        out.push(self.width.qualifier());
        self.width
            .write(u16::try_from(self.controls.len()).unwrap(), out);
        for &(index, status) in &self.controls {
            self.width.write(index, out);
            out.extend_from_slice(self.object.value());
            out.push(status.as_u8());
        }
    }
}

fn request(seq: u8, function: FunctionCode, headers: &[Header]) -> Vec<u8> {
    let mut out = vec![0xC0 | seq, function.as_u8()];
    for header in headers {
        header.write(&mut out);
    }
    out
}

/// response with the restart IIN, IIN2 `iin2` and `headers`
fn response(seq: u8, iin2: u8, headers: &[Header]) -> Vec<u8> {
    let mut out = empty_response(seq, iin2).to_vec();
    for header in headers {
        header.write(&mut out);
    }
    out
}

fn echo(headers: &[Header], mut status: impl FnMut(u16) -> CommandStatus) -> Vec<Header> {
    headers.iter().map(|h| h.with_status(&mut status)).collect()
}

fn controls(headers: &[Header]) -> impl Iterator<Item = Control> + '_ {
    headers.iter().flat_map(|h| {
        h.controls
            .iter()
            .map(|&(index, status)| h.object.control(index, status))
    })
}

fn select_events(headers: &[Header]) -> Vec<Event> {
    let mut events = vec![Event::BeginControls];
    events.extend(controls(headers).map(Event::Select));
    events.push(Event::EndControls);
    events
}

fn operate_events(headers: &[Header], op_type: OperateType) -> Vec<Event> {
    let mut events = vec![Event::BeginControls];
    events.extend(controls(headers).map(|c| Event::Operate(c, op_type)));
    events.push(Event::EndControls);
    events
}

/// SELECT then OPERATE, DIRECT_OPERATE and DIRECT_OPERATE_NO_ACK, each echoed and operated
async fn run_every_function(headers: &[Header]) {
    let mut harness = new_harness(get_default_config());
    let success = echo(headers, |_| Success);

    harness
        .test_request_response(
            &request(0, FunctionCode::Select, headers),
            &response(0, 0, &success),
        )
        .await;
    harness.check_events(&select_events(headers));

    harness
        .test_request_response(
            &request(1, FunctionCode::Operate, headers),
            &response(1, 0, &success),
        )
        .await;
    harness.check_events(&operate_events(headers, OperateType::SelectBeforeOperate));

    harness
        .test_request_response(
            &request(2, FunctionCode::DirectOperate, headers),
            &response(2, 0, &success),
        )
        .await;
    harness.check_events(&operate_events(headers, OperateType::DirectOperate));

    harness
        .send_and_process(&request(3, FunctionCode::DirectOperateNoResponse, headers))
        .await;
    harness.expect_no_response();
    harness.check_events(&operate_events(headers, OperateType::DirectOperateNoAck));
    harness.check_no_events();
}

#[tokio::test]
async fn each_control_object_at_each_width_is_echoed_and_operated_by_every_function() {
    for object in OBJECTS {
        for width in WIDTHS {
            run_every_function(&[Header::pair(object, width)]).await;
        }
    }
}

fn every_object_at_both_widths() -> Vec<Header> {
    WIDTHS
        .into_iter()
        .flat_map(|width| OBJECTS.map(|object| Header::pair(object, width)))
        .collect()
}

#[tokio::test]
async fn every_control_object_at_both_widths_in_one_request_is_echoed_and_operated_in_order() {
    run_every_function(&every_object_at_both_widths()).await;
}

#[tokio::test]
async fn operate_without_a_select_echoes_no_select_for_every_object_and_width() {
    let headers = every_object_at_both_widths();
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &request(0, FunctionCode::Operate, &headers),
            &response(0, 0, &echo(&headers, |_| NoSelect)),
        )
        .await;
    harness.check_no_events();
}

#[tokio::test]
async fn status_in_the_request_reaches_the_handler_and_the_echo_carries_the_handler_status() {
    for object in OBJECTS {
        for width in WIDTHS {
            let header = Header::pair(object, width).with_status(|_| CommandStatus::Local);
            let mut harness = new_harness(get_default_config());
            harness.set_control_status(width.indices()[1], CommandStatus::Unknown(200));

            harness
                .test_request_response(
                    &request(
                        0,
                        FunctionCode::DirectOperate,
                        std::slice::from_ref(&header),
                    ),
                    &response(
                        0,
                        0,
                        &echo(std::slice::from_ref(&header), |i| {
                            if i == width.indices()[1] {
                                CommandStatus::Unknown(200)
                            } else {
                                Success
                            }
                        }),
                    ),
                )
                .await;
            harness.check_events(&operate_events(&[header], OperateType::DirectOperate));
        }
    }
}

/// a one-byte g12v1 header at indices 3 and 4, then a two-byte g41v2 header at 259 and 513
fn two_headers() -> Vec<Header> {
    vec![
        Header::pair(Object::G12V1, Width::One),
        Header::pair(Object::G41V2, Width::Two),
    ]
}

/// The handler returns `statuses` (index, status) for SELECT and DIRECT_OPERATE. Each is echoed per
/// control, `iin2` is the IIN2 of the response, and the select must not be recorded.
async fn handler_statuses(statuses: &[(u16, CommandStatus)], iin2: u8) {
    let headers = two_headers();
    let status = |i| {
        statuses
            .iter()
            .find(|(x, _)| *x == i)
            .map_or(Success, |(_, s)| *s)
    };

    for function in [FunctionCode::DirectOperate, FunctionCode::Select] {
        let mut harness = new_harness(get_default_config());
        for &(index, status) in statuses {
            harness.set_control_status(index, status);
        }

        harness
            .test_request_response(
                &request(0, function, &headers),
                &response(0, iin2, &echo(&headers, status)),
            )
            .await;
        if function == FunctionCode::Select {
            harness.check_events(&select_events(&headers));
            // a select that fails for any control isn't recorded
            harness
                .test_request_response(
                    &request(1, FunctionCode::Operate, &headers),
                    &response(1, 0, &echo(&headers, |_| NoSelect)),
                )
                .await;
        } else {
            harness.check_events(&operate_events(&headers, OperateType::DirectOperate));
        }
        harness.check_no_events();
    }
}

#[tokio::test]
async fn handler_error_is_echoed_without_parameter_error() {
    handler_statuses(&[(4, HardwareError)], 0).await;
}

#[tokio::test]
async fn not_supported_as_the_first_error_sets_parameter_error() {
    handler_statuses(
        &[(4, NotSupported), (259, HardwareError)],
        iin2::PARAMETER_ERROR,
    )
    .await;
    handler_statuses(&[(513, NotSupported)], iin2::PARAMETER_ERROR).await;
}

#[tokio::test]
async fn not_supported_after_another_error_does_not_set_parameter_error() {
    handler_statuses(&[(4, HardwareError), (259, NotSupported)], 0).await;
}

#[tokio::test]
async fn handler_errors_during_operate_are_echoed() {
    let headers = two_headers();
    let mut harness = new_harness(get_default_config());

    harness
        .test_request_response(
            &request(0, FunctionCode::Select, &headers),
            &response(0, 0, &echo(&headers, |_| Success)),
        )
        .await;
    harness.check_events(&select_events(&headers));

    harness.set_control_status(3, NotSupported);
    harness.set_control_status(513, HardwareError);
    harness
        .test_request_response(
            &request(1, FunctionCode::Operate, &headers),
            &response(
                1,
                iin2::PARAMETER_ERROR,
                &echo(&headers, |i| match i {
                    3 => NotSupported,
                    513 => HardwareError,
                    _ => Success,
                }),
            ),
        )
        .await;
    harness.check_events(&operate_events(&headers, OperateType::SelectBeforeOperate));
    harness.check_no_events();
}

fn limited_harness(max: u16) -> OutstationHarness {
    let mut config = get_default_config();
    config.max_controls_per_request = Some(max);
    new_harness(config)
}

/// the first `admitted` controls of `headers`, in order
fn first(headers: &[Header], admitted: usize) -> Vec<Header> {
    let mut remaining = admitted;
    headers
        .iter()
        .map(|h| {
            let take = remaining.min(h.controls.len());
            remaining -= take;
            Header {
                controls: h.controls[..take].to_vec(),
                ..h.clone()
            }
        })
        .filter(|h| !h.controls.is_empty())
        .collect()
}

/// With a limit of `max` controls, the first `max` of the four controls in `two_headers` are
/// handled and the rest are echoed with TOO_MANY_OPS
async fn control_limit(max: u16) {
    let headers = two_headers();
    let admitted = usize::from(max).min(4);
    let echoed = {
        let mut seen = 0;
        echo(&headers, |_| {
            seen += 1;
            if seen <= admitted {
                Success
            } else {
                TooManyOps
            }
        })
    };
    let handled = first(&headers, admitted);
    let events = |events: Vec<Event>| if admitted == 0 { vec![] } else { events };

    // DIRECT_OPERATE
    let mut harness = limited_harness(max);
    harness
        .test_request_response(
            &request(0, FunctionCode::DirectOperate, &headers),
            &response(0, 0, &echoed),
        )
        .await;
    harness.check_events(&events(operate_events(
        &handled,
        OperateType::DirectOperate,
    )));
    harness.check_no_events();

    // DIRECT_OPERATE_NO_ACK
    harness
        .send_and_process(&request(1, FunctionCode::DirectOperateNoResponse, &headers))
        .await;
    harness.expect_no_response();
    harness.check_events(&events(operate_events(
        &handled,
        OperateType::DirectOperateNoAck,
    )));
    harness.check_no_events();

    // SELECT, then OPERATE
    harness
        .test_request_response(
            &request(2, FunctionCode::Select, &headers),
            &response(2, 0, &echoed),
        )
        .await;
    harness.check_events(&events(select_events(&handled)));
    harness.check_no_events();

    if admitted == 4 {
        harness
            .test_request_response(
                &request(3, FunctionCode::Operate, &headers),
                &response(3, 0, &echoed),
            )
            .await;
        harness.check_events(&operate_events(&headers, OperateType::SelectBeforeOperate));
    } else {
        // a select with a control past the limit isn't recorded
        harness
            .test_request_response(
                &request(3, FunctionCode::Operate, &headers),
                &response(3, 0, &echo(&headers, |_| NoSelect)),
            )
            .await;
    }
    harness.check_no_events();
}

#[tokio::test]
async fn control_limit_at_the_number_of_controls_admits_them_all() {
    control_limit(4).await;
}

#[tokio::test]
async fn control_limit_counts_controls_across_headers() {
    control_limit(3).await;
    control_limit(1).await;
}

#[tokio::test]
async fn control_limit_of_zero_handles_no_controls_and_starts_no_fragment() {
    control_limit(0).await;
}
