use scursor::WriteCursor;
use tokio::task::JoinHandle;

use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::parse::options::ParseOptions;
use crate::app::FunctionCode;
use crate::decode::{
    AppDecodeLevel, DecodeLevel, LinkDecodeLevel, PhysDecodeLevel, TransportDecodeLevel,
};
use crate::link::constant::MAX_LINK_FRAME_LENGTH;
use crate::link::format::{format_data_frame, Payload};
use crate::link::header::{AnyAddress, BroadcastConfirmMode, Header};
use crate::link::reader::LinkModes;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::{Add, BinaryInputConfig, EventClass, Update, UpdateOptions};
use crate::outstation::task::OutstationTask;
use crate::outstation::traits::BroadcastAction;
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
        let (mut task, handle) = OutstationTask::<RealTransport>::with_transport(
            Enabled::Yes,
            LinkModes::test(),
            ParseOptions::get_static(),
            config,
            PhysAddr::None,
            application,
            MockOutstationInformation::create(sender.clone()),
            MockControlHandler::create(sender, ControlStatuses::default()),
        );
        let (io, io_handle) = sfio_tokio_mock_io::mock();
        let session = tokio::spawn(async move {
            let mut io = PhysLayer::Mock(io);
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
async fn unknown_function_is_answered_to_whichever_master_sent_it() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        for app in UNKNOWN_FUNCTIONS_SEQ_1 {
            harness.send(&from_master(app)).await;
            harness
                .expect_response(MASTER, NO_FUNCTION_SUPPORT_SEQ_1)
                .await;
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
async fn unknown_function_from_another_master_ends_the_solicited_confirm_wait() {
    for level in decode_levels() {
        let mut harness = LinkHarness::new(get_default_config(), level);
        enter_sol_confirm_wait(&mut harness).await;

        harness
            .send(&request(OUTSTATION, OTHER_MASTER, UNKNOWN_FUNCTION_SEQ_1))
            .await;
        harness
            .expect_response(OTHER_MASTER, &[0xC1, 0x81, 0x82, 0x01])
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

        // an unknown function is answered and the wait goes on
        harness
            .send(&request(OUTSTATION, OTHER_MASTER, UNKNOWN_FUNCTION_SEQ_1))
            .await;
        harness
            .expect_response(OTHER_MASTER, NO_FUNCTION_SUPPORT_SEQ_1)
            .await;
        harness.expect_nothing();

        harness.send(&from_master(UNS_CONFIRM_SEQ_0)).await;
        harness.check_events(&[Event::UnsolicitedConfirmReceived(0)]);
        harness.expect_nothing();
    }
}
