use scursor::WriteCursor;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Instant};

use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::parse::options::ParseOptions;
use crate::app::variations::Group41Var2;
use crate::app::FunctionCode;
use crate::decode::{
    AppDecodeLevel, DecodeLevel, LinkDecodeLevel, PhysDecodeLevel, TransportDecodeLevel,
};
use crate::link::constant::MAX_LINK_FRAME_LENGTH;
use crate::link::format::{format_data_frame, header_bytes, Payload};
use crate::link::header::{AnyAddress, BroadcastConfirmMode, ControlField, Header};
use crate::link::reader::LinkModes;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};
use crate::outstation::task::OutstationTask;
use crate::outstation::traits::{BroadcastAction, OperateType};
use crate::outstation::{BufferState, ClassCount, OutstationHandle, TypeCount};
use crate::transport::RealTransport;
use crate::util::phys::{PhysAddr, PhysLayer};
use crate::util::session::{Enabled, RunError};

use super::harness::*;

/// the addresses in `get_default_config`
const OUTSTATION: u16 = 10;
const MASTER: u16 = 1;
const OTHER_MASTER: u16 = 2;
const OTHER_OUTSTATION: u16 = 11;
const SELF_ADDRESS: u16 = 0xFFFC;

const READ_CLASS_0_SEQ_0: &[u8] = &[0xC0, 0x01, 60, 1, 0x06];
const READ_CLASS_0_SEQ_1: &[u8] = &[0xC1, 0x01, 60, 1, 0x06];
const READ_CLASS_1_SEQ_0: &[u8] = &[0xC0, 0x01, 60, 2, 0x06];
const EMPTY_RESPONSE_SEQ_0: &[u8] = &[0xC0, 0x81, 0x80, 0x00];
const EMPTY_RESPONSE_SEQ_1: &[u8] = &[0xC1, 0x81, 0x80, 0x00];
/// g1v1 with binary input 0, which the confirm-wait tests add
const BINARY_0_CLASS_0_RESPONSE_SEQ_0: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
];
/// a READ whose object header is cut short
const MALFORMED_READ_SEQ_1: &[u8] = &[0xC1, 0x01, 60];
const MALFORMED_RESPONSE_SEQ_1: &[u8] = &[0xC1, 0x81, 0x80, 0x04];
const UNKNOWN_FUNCTION_SEQ_1: &[u8] = &[0xC1, 0x70];
/// 0x83 (AUTHENTICATE_RESPONSE) isn't a function code the outstation knows
const UNKNOWN_FUNCTIONS_SEQ_1: &[&[u8]] = &[UNKNOWN_FUNCTION_SEQ_1, &[0xC1, 0x83, 0x80, 0x00]];
const NO_FUNCTION_SUPPORT_SEQ_1: &[u8] = &[0xC1, 0x81, 0x80, 0x01];
const CONFIRM_SEQ_0: &[u8] = &[0xC0, 0x00];
const UNS_CONFIRM_SEQ_0: &[u8] = &[0xD0, 0x00];
const NULL_UNSOLICITED_SEQ_0: &[u8] = &[0xF0, 0x82, 0x80, 0x00];
/// a fragment too short for a request header, which gets no response
const CONTROL_ONLY_SEQ_1: &[u8] = &[0xC1];
/// REQUEST_LINK_STATUS from the outstation to the master
const LINK_STATUS_REQUEST: &[u8] = &[0x05, 0x64, 0x05, 0x49, 0x01, 0x00, 0x0A, 0x00, 0xDB, 0x9B];
const G41V2_INDEX_7: Control = Control::G41V2(Group41Var2::new(513), 7);
/// g41v2, index 7, value 513
const SELECT_SEQ_0: &[u8] = &[0xC0, 0x03, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
const OPERATE_SEQ_1: &[u8] = &[0xC1, 0x04, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00];
const UNKNOWN_FUNCTION_SEQ_0: &[u8] = &[0xC0, 0x70];
const SELECT_RESPONSE_SEQ_0: &[u8] = &[
    0xC0, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
];
const OPERATE_RESPONSE_SEQ_1: &[u8] = &[
    0xC1, 0x81, 0x80, 0x00, 41, 2, 0x17, 0x01, 0x07, 0x01, 0x02, 0x00,
];

/// fragments that aren't valid requests, each with sequence 1
const NOT_REQUESTS: &[&[u8]] = &[
    // empty fragment
    &[],
    // control field only
    &[0xC1],
    // response function codes
    &[0xC1, 0x81, 0x80, 0x00],
    &[0xF1, 0x82, 0x80, 0x00],
    // response function code without the IIN
    &[0xC1, 0x81, 0x80],
    // FIR without FIN, FIN without FIR, neither
    &[0x81, 0x01, 60, 1, 0x06],
    &[0x41, 0x01, 60, 1, 0x06],
    &[0x01, 0x01, 60, 1, 0x06],
    // UNS set on a READ
    &[0xD1, 0x01, 60, 1, 0x06],
    // a confirm with nothing to confirm
    &[0xC1, 0x00],
    &[0xD1, 0x00],
];

/// application decoding off and fully on, which must not change what the outstation answers
fn decode_levels() -> [DecodeLevel; 2] {
    [
        DecodeLevel::nothing(),
        DecodeLevel::new(
            AppDecodeLevel::ObjectValues,
            TransportDecodeLevel::Payload,
            LinkDecodeLevel::Payload,
            PhysDecodeLevel::Data,
        ),
    ]
}

/// a single-segment fragment in an unconfirmed user data frame
fn frame(
    from_master: bool,
    destination: u16,
    source: u16,
    transport_seq: u8,
    app: &[u8],
) -> Vec<u8> {
    let header = Header::unconfirmed_user_data(
        from_master,
        AnyAddress::from(destination),
        AnyAddress::from(source),
    );
    let mut buffer = [0; MAX_LINK_FRAME_LENGTH];
    let mut cursor = WriteCursor::new(&mut buffer);
    format_data_frame(header, Payload::new(0xC0 | transport_seq, app), &mut cursor)
        .unwrap()
        .frame
        .to_vec()
}

/// a request frame from `source` to `destination`
fn request(destination: u16, source: u16, app: &[u8]) -> Vec<u8> {
    frame(true, destination, source, 0, app)
}

/// a request frame from the configured master to the outstation
fn from_master(app: &[u8]) -> Vec<u8> {
    request(OUTSTATION, MASTER, app)
}

fn add_binary_event(handle: &mut OutstationHandle) {
    handle.database.transaction(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });
}

/// An outstation running over the real link and transport layers
struct LinkHarness {
    io: sfio_tokio_mock_io::Handle,
    events: EventReceiver,
    handle: OutstationHandle,
    /// the transport sequence of the next frame the outstation writes
    tx_seq: u8,
    _session: JoinHandle<RunError>,
}

impl LinkHarness {
    fn new(mut config: OutstationConfig, level: DecodeLevel) -> Self {
        config.decode_level = level;
        let (sender, events) = event_handlers();
        let (_, application) = MockOutstationApplication::create(sender.clone());
        let (_, information) = MockOutstationInformation::create(sender.clone());
        let (mut task, handle) = OutstationTask::<RealTransport>::with_transport(
            Enabled::Yes,
            LinkModes::test(),
            ParseOptions::get_static(),
            config,
            PhysAddr::None,
            application,
            information,
            MockControlHandler::create(sender, ControlStatuses::default()),
        );
        let (io, io_handle) = sfio_tokio_mock_io::mock();
        let session = tokio::spawn(async move {
            let mut io = PhysLayer::mock(io);
            task.run(&mut io).await
        });
        Self {
            io: io_handle,
            events,
            handle,
            tx_seq: 0,
            _session: session,
        }
    }

    async fn send(&mut self, frame: &[u8]) {
        self.io.read(frame);
        assert_eq!(self.io.next_event().await, sfio_tokio_mock_io::Event::Read);
    }

    /// expect the outstation to write `app` to `destination` as its next frame
    async fn expect_response(&mut self, destination: u16, app: &[u8]) {
        let expected = frame(false, destination, OUTSTATION, self.tx_seq, app);
        self.tx_seq = self.tx_seq.wrapping_add(1) & 0x3F;
        assert_eq!(
            self.io.next_event().await,
            sfio_tokio_mock_io::Event::Write(expected)
        );
    }

    fn expect_nothing(&mut self) {
        assert_eq!(self.io.pop_event(), None);
        if let Some(x) = self.events.poll() {
            panic!("expected no events, but next event is: {x:?}")
        }
    }

    fn check_events(&mut self, expected: &[Event]) {
        for event in expected {
            assert_eq!(self.events.poll(), Some(*event));
        }
    }

    async fn send_dropped(&mut self, frame: &[u8]) {
        self.send(frame).await;
        self.expect_nothing();
    }

    /// a READ from the master is still answered normally, without anything left behind by the
    /// frames before it
    async fn probe(&mut self) {
        self.send(&from_master(READ_CLASS_0_SEQ_0)).await;
        self.expect_response(MASTER, EMPTY_RESPONSE_SEQ_0).await;
        self.expect_nothing();
    }
}

#[tokio::test]
async fn request_from_the_master_to_the_outstation_is_answered() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        harness.send(&from_master(READ_CLASS_0_SEQ_1)).await;
        harness.expect_response(MASTER, EMPTY_RESPONSE_SEQ_1).await;
        harness.expect_nothing();
        harness.probe().await;
    }
}

#[tokio::test]
async fn request_to_another_outstation_is_dropped() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        harness
            .send_dropped(&request(OTHER_OUTSTATION, MASTER, READ_CLASS_0_SEQ_1))
            .await;
        harness.probe().await;
    }
}

#[tokio::test]
async fn request_to_the_self_address_is_dropped_unless_enabled() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        harness
            .send_dropped(&request(SELF_ADDRESS, MASTER, READ_CLASS_0_SEQ_1))
            .await;
        harness.probe().await;

        let mut config = get_default_config();
        config.features.self_address = Feature::Enabled;
        let mut harness = LinkHarness::new(config, level);
        harness
            .send(&request(SELF_ADDRESS, MASTER, READ_CLASS_0_SEQ_1))
            .await;
        harness.expect_response(MASTER, EMPTY_RESPONSE_SEQ_1).await;
        harness.probe().await;
    }
}

#[tokio::test]
async fn valid_requests_from_another_master_are_dropped() {
    let requests: &[&[u8]] = &[
        READ_CLASS_0_SEQ_1,
        // malformed objects
        MALFORMED_READ_SEQ_1,
        // confirms
        &[0xC1, 0x00],
        &[0xD1, 0x00],
    ];
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        for app in requests {
            harness
                .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
                .await;
        }
        // a broadcast from another master doesn't set the broadcast IIN
        harness
            .send_dropped(&request(
                BroadcastConfirmMode::NotRequired.address(),
                OTHER_MASTER,
                READ_CLASS_0_SEQ_1,
            ))
            .await;
        harness.probe().await;
    }
}

#[tokio::test]
async fn requests_from_another_master_are_answered_when_any_master_is_allowed() {
    let mut config = get_default_config();
    config.features.respond_to_any_master = Feature::Enabled;
    for level in decode_levels() {
        let mut harness = LinkHarness::new(config, level);
        harness
            .send(&request(OUTSTATION, OTHER_MASTER, READ_CLASS_0_SEQ_1))
            .await;
        harness
            .expect_response(OTHER_MASTER, EMPTY_RESPONSE_SEQ_1)
            .await;
        harness
            .send(&request(OUTSTATION, OTHER_MASTER, &[0xC2, 0x01, 60]))
            .await;
        harness
            .expect_response(OTHER_MASTER, &[0xC2, 0x81, 0x80, 0x04])
            .await;
        harness.expect_nothing();
        harness.probe().await;
    }
}

#[tokio::test]
async fn unknown_function_is_answered_only_from_the_configured_master() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        for app in UNKNOWN_FUNCTIONS_SEQ_1 {
            harness.send(&from_master(app)).await;
            harness
                .expect_response(MASTER, NO_FUNCTION_SUPPORT_SEQ_1)
                .await;
            harness.expect_nothing();
            harness
                .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
                .await;
        }
        harness.probe().await;
    }
}

#[tokio::test]
async fn unknown_function_from_another_master_is_answered_when_any_master_is_allowed() {
    let mut config = get_default_config();
    config.features.respond_to_any_master = Feature::Enabled;
    for level in decode_levels() {
        let mut harness = LinkHarness::new(config, level);
        for app in UNKNOWN_FUNCTIONS_SEQ_1 {
            harness.send(&request(OUTSTATION, OTHER_MASTER, app)).await;
            harness
                .expect_response(OTHER_MASTER, NO_FUNCTION_SUPPORT_SEQ_1)
                .await;
            harness.expect_nothing();
        }
        harness.probe().await;
    }
}

#[tokio::test]
async fn malformed_request_from_the_master_is_answered() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        harness.send(&from_master(MALFORMED_READ_SEQ_1)).await;
        harness
            .expect_response(MASTER, MALFORMED_RESPONSE_SEQ_1)
            .await;
        harness.expect_nothing();
        harness.probe().await;
    }
}

#[tokio::test]
async fn fragments_that_are_not_requests_are_dropped_from_any_master() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        for source in [MASTER, OTHER_MASTER] {
            for app in NOT_REQUESTS {
                harness
                    .send_dropped(&request(OUTSTATION, source, app))
                    .await;
            }
        }
        harness.probe().await;
    }
}

#[tokio::test]
async fn broadcast_gets_no_response_and_sets_the_broadcast_iin() {
    for level in decode_levels() {
        for mode in [
            BroadcastConfirmMode::NotRequired,
            BroadcastConfirmMode::Optional,
        ] {
            let mut harness = LinkHarness::new(get_default_config(), level);
            harness
                .send(&request(mode.address(), MASTER, READ_CLASS_0_SEQ_1))
                .await;
            harness.check_events(&[Event::BroadcastReceived(
                FunctionCode::Read,
                BroadcastAction::UnsupportedFunction(FunctionCode::Read),
            )]);
            harness.expect_nothing();

            harness.send(&from_master(READ_CLASS_0_SEQ_0)).await;
            harness
                .expect_response(MASTER, &[0xC0, 0x81, 0x81, 0x00])
                .await;
            harness.expect_nothing();
            harness.probe().await;
        }

        // a mandatory broadcast asks for a confirm of the next response
        let mut harness = LinkHarness::new(get_default_config(), level);
        harness
            .send(&request(
                BroadcastConfirmMode::Mandatory.address(),
                MASTER,
                READ_CLASS_0_SEQ_1,
            ))
            .await;
        harness.check_events(&[Event::BroadcastReceived(
            FunctionCode::Read,
            BroadcastAction::UnsupportedFunction(FunctionCode::Read),
        )]);
        harness.expect_nothing();

        harness.send(&from_master(READ_CLASS_0_SEQ_0)).await;
        harness
            .expect_response(MASTER, &[0xE0, 0x81, 0x81, 0x00])
            .await;
        harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
        harness.send(&from_master(CONFIRM_SEQ_0)).await;
        harness.check_events(&[
            Event::SolicitedConfirmReceived(0),
            Event::BeginConfirm,
            Event::EndConfirm(empty_buffer()),
        ]);
        harness.expect_nothing();
        harness.send(&from_master(READ_CLASS_0_SEQ_1)).await;
        harness.expect_response(MASTER, EMPTY_RESPONSE_SEQ_1).await;
        harness.expect_nothing();
    }
}

/// READ class 1 with one binary event, which waits for a confirm with sequence 0
async fn enter_sol_confirm_wait(harness: &mut LinkHarness) {
    add_binary_event(&mut harness.handle);
    harness.send(&from_master(READ_CLASS_1_SEQ_0)).await;
    harness
        .expect_response(
            MASTER,
            &[
                0xE0, 0x81, 0x80, 0x00, 0x02, 0x01, 0x28, 0x01, 0x00, 0x00, 0x00, 0x81,
            ],
        )
        .await;
    harness.check_events(&[Event::EnterSolicitedConfirmWait(0)]);
    harness.expect_nothing();
}

fn empty_buffer() -> BufferState {
    BufferState {
        classes: ClassCount {
            num_class_1: 0,
            num_class_2: 0,
            num_class_3: 0,
        },
        types: TypeCount {
            num_binary_input: 0,
            num_double_bit_binary_input: 0,
            num_binary_output_status: 0,
            num_counter: 0,
            num_frozen_counter: 0,
            num_analog: 0,
            num_analog_output_status: 0,
            num_octet_string: 0,
        },
    }
}

fn confirmed_events() -> [Event; 4] {
    [
        Event::SolicitedConfirmReceived(0),
        Event::BeginConfirm,
        Event::Cleared(0),
        Event::EndConfirm(empty_buffer()),
    ]
}

#[tokio::test]
async fn solicited_confirm_wait_drops_requests_from_another_master() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        enter_sol_confirm_wait(&mut harness).await;

        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, CONFIRM_SEQ_0))
            .await;
        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, READ_CLASS_0_SEQ_1))
            .await;
        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, MALFORMED_READ_SEQ_1))
            .await;
        for app in NOT_REQUESTS {
            harness
                .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
                .await;
        }

        harness.send(&from_master(CONFIRM_SEQ_0)).await;
        harness.check_events(&confirmed_events());
        harness.expect_nothing();
        harness.send(&from_master(READ_CLASS_0_SEQ_0)).await;
        harness
            .expect_response(MASTER, BINARY_0_CLASS_0_RESPONSE_SEQ_0)
            .await;
        harness.expect_nothing();
    }
}

#[tokio::test]
async fn unknown_function_from_another_master_does_not_end_the_solicited_confirm_wait() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        enter_sol_confirm_wait(&mut harness).await;

        for app in UNKNOWN_FUNCTIONS_SEQ_1 {
            harness
                .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
                .await;
        }

        harness.send(&from_master(CONFIRM_SEQ_0)).await;
        harness.check_events(&confirmed_events());
        harness.expect_nothing();
        harness.send(&from_master(READ_CLASS_0_SEQ_0)).await;
        harness
            .expect_response(MASTER, BINARY_0_CLASS_0_RESPONSE_SEQ_0)
            .await;
        harness.expect_nothing();
    }
}

#[tokio::test]
async fn unknown_function_from_the_master_ends_the_solicited_confirm_wait() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        enter_sol_confirm_wait(&mut harness).await;

        harness.send(&from_master(UNKNOWN_FUNCTION_SEQ_1)).await;
        harness
            .expect_response(MASTER, &[0xC1, 0x81, 0x82, 0x01])
            .await;
        harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
        harness.expect_nothing();

        // the wait is over
        harness.send_dropped(&from_master(CONFIRM_SEQ_0)).await;
    }
}

#[tokio::test]
async fn request_from_the_master_ends_the_solicited_confirm_wait_and_is_answered() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        enter_sol_confirm_wait(&mut harness).await;

        harness.send(&from_master(READ_CLASS_0_SEQ_1)).await;
        harness.check_events(&[Event::SolicitedConfirmWaitNewRequest]);
        harness
            .expect_response(
                MASTER,
                &[
                    0xC1, 0x81, 0x82, 0x00, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
                ],
            )
            .await;
        harness.expect_nothing();
    }
}

#[tokio::test]
async fn unsolicited_confirm_wait_drops_requests_from_another_master() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_unsolicited_config(), level);
        harness
            .expect_response(MASTER, NULL_UNSOLICITED_SEQ_0)
            .await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);

        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, UNS_CONFIRM_SEQ_0))
            .await;
        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, READ_CLASS_0_SEQ_1))
            .await;
        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, MALFORMED_READ_SEQ_1))
            .await;
        for app in NOT_REQUESTS {
            harness
                .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
                .await;
        }

        for app in UNKNOWN_FUNCTIONS_SEQ_1 {
            harness
                .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
                .await;
        }

        harness.send(&from_master(UNS_CONFIRM_SEQ_0)).await;
        harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
        harness.expect_nothing();
        harness.probe().await;
    }
}

const KEEP_ALIVE_TIMEOUT: Duration = Duration::from_secs(10);
const ACTIVITY_AT: Duration = Duration::from_secs(4);

/// send `frame` at 4 seconds, expecting `response`, and return when the link status request goes out
async fn link_status_request_after(
    mut config: OutstationConfig,
    level: DecodeLevel,
    frame: &[u8],
    response: Option<(u16, &[u8])>,
) -> Duration {
    config.keep_alive_timeout = Some(KEEP_ALIVE_TIMEOUT);
    let start = Instant::now();
    let mut harness = LinkHarness::new(config, level);
    tokio::time::advance(ACTIVITY_AT).await;
    harness.send(frame).await;
    if let Some((destination, app)) = response {
        harness.expect_response(destination, app).await;
    }
    harness.expect_nothing();
    assert_eq!(
        harness.io.next_event().await,
        sfio_tokio_mock_io::Event::Write(LINK_STATUS_REQUEST.to_vec())
    );
    start.elapsed()
}

#[tokio::test(start_paused = true)]
async fn header_error_from_another_master_does_not_postpone_the_link_status_request() {
    for level in decode_levels() {
        for app in [UNKNOWN_FUNCTION_SEQ_1, CONTROL_ONLY_SEQ_1] {
            assert_eq!(
                link_status_request_after(
                    get_default_config(),
                    level,
                    &request(OUTSTATION, OTHER_MASTER, app),
                    None,
                )
                .await,
                KEEP_ALIVE_TIMEOUT
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn header_error_from_the_master_postpones_the_link_status_request() {
    for level in decode_levels() {
        for (app, response) in [
            (
                UNKNOWN_FUNCTION_SEQ_1,
                Some((MASTER, NO_FUNCTION_SUPPORT_SEQ_1)),
            ),
            (CONTROL_ONLY_SEQ_1, None),
        ] {
            assert_eq!(
                link_status_request_after(get_default_config(), level, &from_master(app), response)
                    .await,
                ACTIVITY_AT + KEEP_ALIVE_TIMEOUT
            );
        }
    }
}

#[tokio::test(start_paused = true)]
async fn header_error_from_another_master_postpones_the_link_status_request_when_any_master_is_allowed(
) {
    let mut config = get_default_config();
    config.features.respond_to_any_master = Feature::Enabled;
    for level in decode_levels() {
        for (app, response) in [
            (
                UNKNOWN_FUNCTION_SEQ_1,
                Some((OTHER_MASTER, NO_FUNCTION_SUPPORT_SEQ_1)),
            ),
            (CONTROL_ONLY_SEQ_1, None),
        ] {
            assert_eq!(
                link_status_request_after(
                    config,
                    level,
                    &request(OUTSTATION, OTHER_MASTER, app),
                    response,
                )
                .await,
                ACTIVITY_AT + KEEP_ALIVE_TIMEOUT
            );
        }
    }
}

/// SELECT from the master, then an unknown function from another master, then the OPERATE
async fn unknown_function_from_another_master_keeps_the_selection(harness: &mut LinkHarness) {
    harness.send(&from_master(SELECT_SEQ_0)).await;
    harness.expect_response(MASTER, SELECT_RESPONSE_SEQ_0).await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Select(G41V2_INDEX_7),
        Event::EndControls,
    ]);
    harness.expect_nothing();

    for app in [UNKNOWN_FUNCTION_SEQ_0, UNKNOWN_FUNCTION_SEQ_1] {
        harness
            .send_dropped(&request(OUTSTATION, OTHER_MASTER, app))
            .await;
    }

    harness.send(&from_master(OPERATE_SEQ_1)).await;
    harness
        .expect_response(MASTER, OPERATE_RESPONSE_SEQ_1)
        .await;
    harness.check_events(&[
        Event::BeginControls,
        Event::Operate(G41V2_INDEX_7, OperateType::SelectBeforeOperate),
        Event::EndControls,
    ]);
    harness.expect_nothing();
}

#[tokio::test]
async fn unknown_function_from_another_master_between_select_and_operate_keeps_the_selection() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        unknown_function_from_another_master_keeps_the_selection(&mut harness).await;
    }
}

#[tokio::test]
async fn unknown_function_from_another_master_keeps_the_selection_in_the_unsolicited_confirm_wait()
{
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_unsolicited_config(), level);
        harness
            .expect_response(MASTER, NULL_UNSOLICITED_SEQ_0)
            .await;
        harness.check_events(&[Event::EnterUnsolicitedConfirmWait(0)]);
        harness.expect_nothing();

        unknown_function_from_another_master_keeps_the_selection(&mut harness).await;

        harness.send(&from_master(UNS_CONFIRM_SEQ_0)).await;
        harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
        harness.expect_nothing();
    }
}

/// a header-only frame with the link control octet `control`
fn header_only(control: u8, destination: u16, source: u16) -> Vec<u8> {
    header_bytes(
        Header::new(
            ControlField::from(control),
            AnyAddress::from(destination),
            AnyAddress::from(source),
        ),
        5,
    )
    .to_vec()
}

/// REQUEST_LINK_STATUS from the master, and the outstation's LINK_STATUS reply
fn link_status_exchange() -> (Vec<u8>, Vec<u8>) {
    (
        header_only(0xC9, OUTSTATION, MASTER),
        header_only(0x0B, MASTER, OUTSTATION),
    )
}

impl LinkHarness {
    /// send the master's REQUEST_LINK_STATUS and expect the reply
    async fn exchange_link_status(&mut self) {
        let (request, reply) = link_status_exchange();
        self.send(&request).await;
        assert_eq!(
            self.io.next_event().await,
            sfio_tokio_mock_io::Event::Write(reply)
        );
    }

    async fn expect_link_status_request(&mut self) {
        assert_eq!(
            self.io.next_event().await,
            sfio_tokio_mock_io::Event::Write(LINK_STATUS_REQUEST.to_vec())
        );
    }
}

fn with_keep_alive(mut config: OutstationConfig) -> OutstationConfig {
    config.keep_alive_timeout = Some(KEEP_ALIVE_TIMEOUT);
    config
}

#[tokio::test(start_paused = true)]
async fn link_status_request_from_the_master_postpones_the_link_status_request() {
    let start = Instant::now();
    let mut harness = LinkHarness::new(
        with_keep_alive(get_default_config()),
        DecodeLevel::nothing(),
    );
    tokio::time::advance(ACTIVITY_AT).await;
    harness.exchange_link_status().await;
    harness.expect_nothing();
    harness.expect_link_status_request().await;
    assert_eq!(start.elapsed(), ACTIVITY_AT + KEEP_ALIVE_TIMEOUT);
}

#[tokio::test(start_paused = true)]
async fn link_status_request_from_the_master_in_the_solicited_confirm_wait_postpones_the_link_status_request(
) {
    let start = Instant::now();
    let mut harness = LinkHarness::new(
        with_keep_alive(get_default_config()),
        DecodeLevel::nothing(),
    );
    enter_sol_confirm_wait(&mut harness).await;
    tokio::time::advance(ACTIVITY_AT).await;
    harness.exchange_link_status().await;
    harness.expect_nothing();

    // the confirm wait times out at 5 seconds, before the link status request is due
    harness.expect_link_status_request().await;
    harness.check_events(&[Event::SolicitedConfirmTimeout(0)]);
    assert_eq!(start.elapsed(), ACTIVITY_AT + KEEP_ALIVE_TIMEOUT);
}

/// the null unsolicited response is sent again every 5 seconds, and the link status request is
/// only checked between the waits
#[tokio::test(start_paused = true)]
async fn link_status_request_from_the_master_in_the_unsolicited_confirm_wait_postpones_the_link_status_request(
) {
    let mut harness = LinkHarness::new(
        with_keep_alive(get_default_unsolicited_config()),
        DecodeLevel::nothing(),
    );
    harness
        .expect_response(MASTER, NULL_UNSOLICITED_SEQ_0)
        .await;
    tokio::time::advance(ACTIVITY_AT).await;
    harness.exchange_link_status().await;

    // due at 14 seconds, so after the wait that ends at 15, not the one that ends at 10
    harness
        .expect_response(MASTER, &[0xF1, 0x82, 0x80, 0x00])
        .await;
    harness
        .expect_response(MASTER, &[0xF2, 0x82, 0x80, 0x00])
        .await;
    harness.expect_link_status_request().await;
}
