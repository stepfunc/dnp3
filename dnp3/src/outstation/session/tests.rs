// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Unit tests for [`OutstationCore`]'s sans-I/O contract (in-flight drop recovery on abandoned
//! `Transmit` and `handle_input`, confirm-wait tracking, and callback DB-lock reentrancy) and
//! cancellable-write tests on [`OutstationTask`].

use std::cell::Cell;
use std::collections::BTreeSet;
use std::pin::pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::Duration;

use crate::app::attr::{AttrProp, AttrSet, Attribute, OwnedAttrValue, OwnedAttribute, StringAttr};
use crate::app::control::CommandStatus;
use crate::app::measurement::{BinaryInput, Flags, Time};
use crate::app::{
    BufferSize, FunctionCode, MaybeAsync, RequestHeader, Sequence, Timeout, Timestamp,
};
use crate::decode::AppDecodeLevel;
use crate::link::header::BroadcastConfirmMode;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::config::{Feature, OutstationConfig};
use crate::outstation::database::{
    Add, AnalogInputConfig, BinaryInputConfig, CounterConfig, DatabaseHandle,
    EventBinaryInputVariation, EventBufferConfig, EventClass, Update, UpdateOptions,
};
use crate::outstation::session::core::{Outbound, Output, OutstationCore};
use crate::outstation::session::state::ConfirmWaitInfo;
use crate::outstation::session::task::OutstationTask;
use crate::outstation::traits::{
    ApplicationIin, BroadcastAction, BufferState, ControlHandler, ControlSupport,
    DefaultControlHandler, FreezeIndices, FreezeType, OperateType, OutstationApplication,
    OutstationInformation, PointIndices, PointType, RequestError, RestartDelay,
};
use crate::outstation::OutstationHandle;
use crate::phys::{PhysAddr, PhysLayer};
use crate::transport::{
    Fragment, FragmentAddr, FragmentInfo, LinkLayerMessage, LinkLayerMessageType, TransportData,
};
use crate::util::clock::{MonotonicClock, MonotonicTime};
use crate::util::lifecycle::{Enabled, RunError, StopReason};
use crate::util::test_support::{poll_once, watchdog, NullApplication, NullInformation};

const OUTSTATION: EndpointAddress = EndpointAddress::raw(10);
const MASTER: EndpointAddress = EndpointAddress::raw(1);

struct TestClock(Cell<MonotonicTime>);

impl TestClock {
    fn new() -> Self {
        Self(Cell::new(MonotonicTime::from_elapsed(Duration::ZERO)))
    }

    fn advance(&self, delta: Duration) -> MonotonicTime {
        let next = self.0.get().saturating_add(delta);
        self.0.set(next);
        next
    }
}

impl MonotonicClock for TestClock {
    fn now(&self) -> MonotonicTime {
        self.0.get()
    }
}

fn frag_addr() -> FragmentAddr {
    FragmentAddr {
        link: MASTER,
        phys: PhysAddr::None,
    }
}

fn input_fragment(data: &[u8]) -> TransportData<'_> {
    TransportData::Fragment(Fragment {
        info: FragmentInfo::new(frag_addr(), None),
        data,
    })
}

fn broadcast_fragment(data: &[u8], mode: BroadcastConfirmMode) -> TransportData<'_> {
    TransportData::Fragment(Fragment {
        info: FragmentInfo::new(frag_addr(), Some(mode)),
        data,
    })
}

fn base_config() -> OutstationConfig {
    let mut config = OutstationConfig::new(OUTSTATION, MASTER, EventBufferConfig::all_types(10));
    config.features.unsolicited = Feature::Disabled;
    config.keep_alive_timeout = None;
    config.confirm_timeout = Timeout::from_secs(5).unwrap();
    config
}

fn new_db(config: &OutstationConfig) -> DatabaseHandle {
    DatabaseHandle::new(
        config.read_header_limit(),
        config.class_zero,
        config.event_buffer_config,
    )
}

fn add_class_1_binary_event(db: &mut DatabaseHandle) {
    db.with_database(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });
}

#[derive(Debug, PartialEq, Eq)]
enum InfoEvent {
    RequestFromIdle(FunctionCode, u8),
    EnterSolWait(u8),
}

#[derive(Clone, Default)]
struct RecordingInfo(Arc<Mutex<Vec<InfoEvent>>>);

impl RecordingInfo {
    fn take(&self) -> Vec<InfoEvent> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }
}

impl OutstationInformation for RecordingInfo {
    fn process_request_from_idle(&mut self, header: RequestHeader) {
        self.0.lock().unwrap().push(InfoEvent::RequestFromIdle(
            header.function,
            header.control.seq.value(),
        ));
    }

    fn enter_solicited_confirm_wait(&mut self, ecsn: Sequence) {
        self.0
            .lock()
            .unwrap()
            .push(InfoEvent::EnterSolWait(ecsn.value()));
    }
}

/// Drives a `handle_input` future that resolves in a single poll without a Tokio runtime.
fn run_sync_input(core: &mut OutstationCore<&TestClock>, input: TransportData<'_>) {
    let fut = pin!(core.handle_input(input, AppDecodeLevel::Nothing));
    assert!(matches!(poll_once(fut), Poll::Ready(())));
}

fn send_expect_fragment(core: &mut OutstationCore<&TestClock>) -> Vec<u8> {
    match core.poll() {
        Output::Transmit(tx) => {
            let bytes = match tx.outbound() {
                Outbound::Fragment { destination, bytes } => {
                    assert_eq!(destination.link, MASTER);
                    bytes.to_vec()
                }
                Outbound::LinkStatusRequest { .. } => panic!("unexpected link status"),
            };
            tx.sent();
            bytes
        }
        Output::Wait(_) => panic!("expected staged fragment"),
    }
}

#[test]
fn transmit_sent_arms_confirm_deadline_and_dropping_transmit_resets() {
    let clock = TestClock::new();
    let config = base_config();
    let info = RecordingInfo::default();
    let mut db = new_db(&config);
    add_class_1_binary_event(&mut db);

    let mut core = OutstationCore::new(
        &config,
        PhysAddr::None,
        db.clone(),
        Box::new(NullApplication),
        Box::new(info.clone()),
        DefaultControlHandler::create(),
        &clock,
    );

    assert!(matches!(core.poll(), Output::Wait(None)));

    // READ class 1 (g60v2, qualifier 0x06), seq 0 -> stages a response requiring confirm.
    let read_class_1 = [0xC0, 0x01, 60, 2, 0x06];
    run_sync_input(&mut core, input_fragment(&read_class_1));

    // Drop `Transmit` without calling `sent` (simulating an aborted write without `reset`).
    {
        let Output::Transmit(tx) = core.poll() else {
            panic!("expected staged response");
        };
        match tx.outbound() {
            Outbound::Fragment { destination, bytes } => {
                assert_eq!(destination.link, MASTER);
                assert_eq!(&bytes[..4], &[0xE0, 0x81, 0x80, 0x00]);
            }
            Outbound::LinkStatusRequest { .. } => panic!("unexpected link status"),
        }
    }

    // Confirm wait was NOT entered, and the Class 1 event is still Written until recovery runs.
    assert_eq!(core.confirm_wait_info(), None);
    assert_eq!(
        info.take(),
        vec![InfoEvent::RequestFromIdle(FunctionCode::Read, 0)]
    );
    assert!(!db.get_events_info().unwritten_classes.class1);

    // Next `poll` detects `InFlight`, resets per-connection state and DB (restoring the Class 1
    // event to Unselected), and returns `Wait`.
    assert!(matches!(core.poll(), Output::Wait(None)));
    assert!(db.get_events_info().unwritten_classes.class1);

    // A fresh READ gets the restored event; calling `sent()` arms the deadline from `t1`.
    run_sync_input(&mut core, input_fragment(&read_class_1));
    let t1 = clock.advance(Duration::from_secs(1));
    let sent_bytes = send_expect_fragment(&mut core);
    assert_eq!(&sent_bytes[..4], &[0xE0, 0x81, 0x80, 0x00]);

    assert_eq!(
        core.confirm_wait_info(),
        Some(ConfirmWaitInfo::Solicited(Sequence::new(0)))
    );
    assert_eq!(
        info.take(),
        vec![
            InfoEvent::RequestFromIdle(FunctionCode::Read, 0),
            InfoEvent::EnterSolWait(0),
        ]
    );

    let expected_deadline = t1.saturating_add(Duration::from_secs(5));
    assert!(matches!(core.poll(), Output::Wait(Some(d)) if d == expected_deadline));
}

#[test]
fn multi_fragment_continuation_updates_confirm_wait_and_survives_noop_input() {
    let clock = TestClock::new();
    let mut config = base_config();
    config.event_buffer_config = EventBufferConfig::all_types(40);
    config.solicited_buffer_size = BufferSize::min();

    let db = new_db(&config);
    db.with_database(|db| {
        db.add(
            0,
            Some(EventClass::Class1),
            BinaryInputConfig {
                e_var: EventBinaryInputVariation::Group2Var2,
                ..BinaryInputConfig::default()
            },
        );
        for i in 0..35u64 {
            db.update(
                0,
                &BinaryInput::new(i % 2 == 0, Flags::ONLINE, Time::synchronized(i)),
                UpdateOptions::default(),
            );
        }
    });

    let mut core = OutstationCore::new(
        &config,
        PhysAddr::None,
        db,
        Box::new(NullApplication),
        Box::new(NullInformation),
        DefaultControlHandler::create(),
        &clock,
    );

    // READ class 1 (seq 0) spans two fragments because 35 g2v2 events exceed 249 bytes.
    let read_class_1 = [0xC0, 0x01, 60, 2, 0x06];
    run_sync_input(&mut core, input_fragment(&read_class_1));
    let frag0 = send_expect_fragment(&mut core);
    assert_eq!(&frag0[..4], &[0xA0, 0x81, 0x82, 0x00]); // FIR=1, FIN=0, CON=1, seq=0, CLASS_1
    assert_eq!(
        core.confirm_wait_info(),
        Some(ConfirmWaitInfo::Solicited(Sequence::new(0)))
    );

    // Confirm fragment 0 -> stages continuation fragment 1 (seq 1, FIN=1, CON=1) with updated ecsn.
    run_sync_input(&mut core, input_fragment(&[0xC0, 0x00]));
    assert_eq!(
        core.confirm_wait_info(),
        Some(ConfirmWaitInfo::Solicited(Sequence::new(1)))
    );

    // A no-op input (link-layer message) must NOT discard the staged continuation.
    let link_msg = TransportData::LinkLayerMessage(LinkLayerMessage {
        source: MASTER,
        message: LinkLayerMessageType::LinkStatusResponse,
    });
    run_sync_input(&mut core, link_msg);
    let frag1 = send_expect_fragment(&mut core);
    assert_eq!(&frag1[..4], &[0x61, 0x81, 0x80, 0x00]); // FIR=0, FIN=1, CON=1, seq=1

    // A duplicate Confirm(0) must be ignored (ecsn is already 1), not double-confirming fragment 0.
    run_sync_input(&mut core, input_fragment(&[0xC0, 0x00]));
    assert_eq!(
        core.confirm_wait_info(),
        Some(ConfirmWaitInfo::Solicited(Sequence::new(1)))
    );

    // A repeat READ re-stages fragment 1 and keeps ecsn = 1 aligned with the published header.
    run_sync_input(&mut core, input_fragment(&read_class_1));
    let resent = send_expect_fragment(&mut core);
    assert_eq!(resent, frag1);
    assert_eq!(
        core.confirm_wait_info(),
        Some(ConfirmWaitInfo::Solicited(Sequence::new(1)))
    );
}

#[test]
fn static_final_continuation_fragment_does_not_hold_confirm_wait() {
    let clock = TestClock::new();
    let mut config = base_config();
    config.solicited_buffer_size = BufferSize::min();

    let db = new_db(&config);
    db.with_database(|db| {
        for idx in 0..60u16 {
            db.add(idx, None, AnalogInputConfig::default());
        }
    });

    let mut core = OutstationCore::new(
        &config,
        PhysAddr::None,
        db,
        Box::new(NullApplication),
        Box::new(NullInformation),
        DefaultControlHandler::create(),
        &clock,
    );

    // READ class 0 (60 g30v1 points = 300 bytes) spans two static-only fragments.
    run_sync_input(&mut core, input_fragment(&[0xC0, 0x01, 60, 1, 0x06]));
    let frag0 = send_expect_fragment(&mut core);
    assert_eq!(&frag0[..4], &[0xA0, 0x81, 0x80, 0x00]); // FIR=1, FIN=0, CON=1, seq=0

    // Confirm fragment 0 -> stages final static fragment (FIR=0, FIN=1, CON=0, seq=1) with no wait.
    run_sync_input(&mut core, input_fragment(&[0xC0, 0x00]));
    assert_eq!(core.confirm_wait_info(), None);

    let frag1 = send_expect_fragment(&mut core);
    assert_eq!(&frag1[..4], &[0x41, 0x81, 0x80, 0x00]); // FIR=0, FIN=1, CON=0, seq=1
    assert_eq!(core.confirm_wait_info(), None);
    assert!(matches!(core.poll(), Output::Wait(None)));
}

struct PendingAttrApp;

impl OutstationApplication for PendingAttrApp {
    fn write_device_attr(&mut self, _attr: Attribute) -> MaybeAsync<bool> {
        MaybeAsync::asynchronous(std::future::pending())
    }
}

#[derive(Copy, Clone)]
enum RecoverVia {
    Poll,
    HandleInput,
}

#[test]
fn dropped_in_flight_handle_input_resets_core_and_db_on_next_poll_and_input() {
    for trigger in [RecoverVia::Poll, RecoverVia::HandleInput] {
        let clock = TestClock::new();
        let mut config = base_config();
        config.features.unsolicited = Feature::Enabled;
        let mut db = new_db(&config);
        add_class_1_binary_event(&mut db);
        db.with_database(|db| {
            db.define_attr(
                AttrProp::writable(),
                OwnedAttribute::new(
                    AttrSet::Default,
                    246,
                    OwnedAttrValue::VisibleString("init".to_string()),
                ),
            )
            .unwrap();
        });

        let mut core = OutstationCore::new(
            &config,
            PhysAddr::None,
            db.clone(),
            Box::new(PendingAttrApp),
            Box::new(NullInformation),
            DefaultControlHandler::create(),
            &clock,
        );

        // Complete NULL unsolicited, enable Class 1 unsolicited, and enter UnsolConfirmWait(1)
        // with the Class 1 event in the Written state (`!unwritten_classes.class1`).
        let null_unsol = send_expect_fragment(&mut core);
        assert_eq!(null_unsol, [0xF0, 0x82, 0x82, 0x00]);
        run_sync_input(&mut core, input_fragment(&[0xD0, 0x00]));

        // ENABLE_UNSOLICITED class 1 (g60v2, qualifier 0x06), seq 0.
        run_sync_input(&mut core, input_fragment(&[0xC0, 0x14, 60, 2, 0x06]));
        let enable_resp = send_expect_fragment(&mut core);
        assert_eq!(enable_resp, [0xC0, 0x81, 0x82, 0x00]);

        // Unsolicited event response (seq 1) moves the Class 1 event out of unwritten.
        let event_unsol = send_expect_fragment(&mut core);
        assert_eq!(&event_unsol[..4], &[0xF1, 0x82, 0x80, 0x00]);
        assert_eq!(
            core.confirm_wait_info(),
            Some(ConfirmWaitInfo::Unsolicited(Sequence::new(1)))
        );
        assert!(!db.get_events_info().unwritten_classes.class1);

        // Start a WRITE g0v246 ("hi") during UnsolConfirmWait that suspends in `write_device_attr`,
        // then drop the future mid-await.
        let write_attr = [0xC1, 0x02, 0, 246, 0x00, 0x00, 0x00, 0x01, 2, b'h', b'i'];
        {
            let fut = pin!(core.handle_input(input_fragment(&write_attr), AppDecodeLevel::Nothing));
            assert!(matches!(poll_once(fut), Poll::Pending));
        }

        // Before recovery runs: DB still holds the unconfirmed event as Written (not yet reset),
        // and `phase` is `InFlight` (`confirm_wait_info()` is `None`).
        assert!(!db.get_events_info().unwritten_classes.class1);
        assert_eq!(core.confirm_wait_info(), None);

        let expected_event_payload = [2, 1, 0x28, 1, 0, 0, 0, 0x81];
        match trigger {
            RecoverVia::Poll => {
                // `poll` recovers, resets DB (restoring the Class 1 event), and stages a fresh
                // unsolicited response carrying the restored event (seq 2).
                let recovered_unsol = send_expect_fragment(&mut core);
                assert_eq!(&recovered_unsol[..4], &[0xF2, 0x82, 0x80, 0x00]);
                assert_eq!(&recovered_unsol[4..], &expected_event_payload);
            }
            RecoverVia::HandleInput => {
                // `handle_input` recovers and resets DB before processing the incoming READ class 1,
                // so the READ reports the restored event.
                run_sync_input(&mut core, input_fragment(&[0xC2, 0x01, 60, 2, 0x06]));
                let read_resp = send_expect_fragment(&mut core);
                assert_eq!(&read_resp[..4], &[0xE2, 0x81, 0x80, 0x00]);
                assert_eq!(&read_resp[4..], &expected_event_payload);
            }
        }
    }
}

// ---------- Callback locking reentrancy test ----------

/// Every outside-the-lock callback calls `transaction` on a cloned `DatabaseHandle` and records
/// its name in `called`, proving none of them hold the DB mutex.
#[derive(Clone)]
struct ReentrantCallbacks {
    db: DatabaseHandle,
    called: Arc<Mutex<BTreeSet<&'static str>>>,
}

impl ReentrantCallbacks {
    fn new(db: DatabaseHandle, called: Arc<Mutex<BTreeSet<&'static str>>>) -> Self {
        Self { db, called }
    }

    fn touch_db(&self, name: &'static str) {
        // Acquiring the DB mutex here deadlocks if the outstation called this callback while
        // holding the database lock.
        self.db.transaction(|_| {});
        self.called.lock().unwrap().insert(name);
    }
}

impl OutstationApplication for ReentrantCallbacks {
    fn get_processing_delay_ms(&self) -> u16 {
        self.touch_db("get_processing_delay_ms");
        0
    }

    fn write_absolute_time(&mut self, _time: Timestamp) -> Result<(), RequestError> {
        self.touch_db("write_absolute_time");
        Ok(())
    }

    fn get_application_iin(&self) -> ApplicationIin {
        self.touch_db("get_application_iin");
        ApplicationIin::default()
    }

    fn cold_restart(&mut self) -> Option<RestartDelay> {
        self.touch_db("cold_restart");
        Some(RestartDelay::Milliseconds(10))
    }

    fn warm_restart(&mut self) -> Option<RestartDelay> {
        self.touch_db("warm_restart");
        Some(RestartDelay::Milliseconds(10))
    }

    fn freeze_counter(
        &mut self,
        _indices: FreezeIndices,
        _freeze_type: FreezeType,
        _database: &mut DatabaseHandle,
    ) -> Result<(), RequestError> {
        self.touch_db("freeze_counter");
        Ok(())
    }

    fn support_write_analog_dead_bands(&mut self) -> bool {
        self.touch_db("support_write_analog_dead_bands");
        true
    }

    fn begin_write_analog_dead_bands(&mut self) {
        self.touch_db("begin_write_analog_dead_bands");
    }

    fn write_analog_dead_band(&mut self, _index: u16, _dead_band: f64) {
        // Explicitly inside the DB lock; do not re-lock here.
        self.called.lock().unwrap().insert("write_analog_dead_band");
    }

    fn end_write_analog_dead_bands(&mut self) -> MaybeAsync<()> {
        self.touch_db("end_write_analog_dead_bands");
        MaybeAsync::ready(())
    }

    fn support_assign_class(&mut self) -> bool {
        self.touch_db("support_assign_class");
        true
    }

    fn begin_assign_class(&mut self) {
        self.touch_db("begin_assign_class");
    }

    fn assign_class(
        &mut self,
        _point_type: PointType,
        _indices: PointIndices,
        _class: Option<EventClass>,
    ) {
        self.touch_db("assign_class");
    }

    fn end_assign_class(&mut self) -> MaybeAsync<()> {
        self.touch_db("end_assign_class");
        MaybeAsync::ready(())
    }

    fn write_device_attr(&mut self, _attr: Attribute) -> MaybeAsync<bool> {
        self.touch_db("write_device_attr");
        MaybeAsync::ready(true)
    }

    fn begin_confirm(&mut self) {
        self.touch_db("begin_confirm");
    }

    fn event_cleared(&mut self, _id: u64) {
        // Explicitly inside the DB lock; do not re-lock here.
        self.called.lock().unwrap().insert("event_cleared");
    }

    fn end_confirm(&mut self, _state: BufferState) -> MaybeAsync<()> {
        self.touch_db("end_confirm");
        MaybeAsync::ready(())
    }
}

impl OutstationInformation for ReentrantCallbacks {
    fn process_request_from_idle(&mut self, _header: RequestHeader) {
        self.touch_db("process_request_from_idle");
    }

    fn broadcast_received(&mut self, _function: FunctionCode, _action: BroadcastAction) {
        self.touch_db("broadcast_received");
    }

    fn enter_solicited_confirm_wait(&mut self, _ecsn: Sequence) {
        self.touch_db("enter_solicited_confirm_wait");
    }

    fn solicited_confirm_timeout(&mut self, _ecsn: Sequence) {
        self.touch_db("solicited_confirm_timeout");
    }

    fn solicited_confirm_received(&mut self, _ecsn: Sequence) {
        self.touch_db("solicited_confirm_received");
    }

    fn solicited_confirm_wait_new_request(&mut self) {
        self.touch_db("solicited_confirm_wait_new_request");
    }

    fn wrong_solicited_confirm_seq(&mut self, _ecsn: Sequence, _seq: Sequence) {
        self.touch_db("wrong_solicited_confirm_seq");
    }

    fn unexpected_confirm(&mut self, _unsolicited: bool, _seq: Sequence) {
        self.touch_db("unexpected_confirm");
    }

    fn enter_unsolicited_confirm_wait(&mut self, _ecsn: Sequence) {
        self.touch_db("enter_unsolicited_confirm_wait");
    }

    fn unsolicited_confirm_timeout(&mut self, _ecsn: Sequence, _retry: bool) {
        self.touch_db("unsolicited_confirm_timeout");
    }

    fn unsolicited_confirmed(&mut self, _ecsn: Sequence) {
        self.touch_db("unsolicited_confirmed");
    }

    fn clear_restart_iin(&mut self) {
        self.touch_db("clear_restart_iin");
    }
}

impl ControlHandler for ReentrantCallbacks {
    fn begin_fragment(&mut self) {
        self.touch_db("control_begin_fragment");
    }

    fn end_fragment(&mut self, _database: &mut DatabaseHandle) -> MaybeAsync<()> {
        self.touch_db("control_end_fragment");
        MaybeAsync::ready(())
    }
}

impl<T> ControlSupport<T> for ReentrantCallbacks {
    fn select(
        &mut self,
        _control: T,
        _index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.touch_db("control_select");
        CommandStatus::Success
    }

    fn operate(
        &mut self,
        _control: T,
        _index: u16,
        _op_type: OperateType,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.touch_db("control_operate");
        CommandStatus::Success
    }
}

fn exchange_ok(core: &mut OutstationCore<&TestClock>, req: &[u8]) {
    run_sync_input(core, input_fragment(req));
    let resp = send_expect_fragment(core);
    assert!(
        resp.len() >= 4 && resp[3] == 0x00,
        "unexpected response or non-zero IIN2: {resp:02X?}"
    );
}

#[test]
fn outside_the_lock_callbacks_can_transact_on_database_handle_without_deadlock() {
    let clock = TestClock::new();
    let mut config = base_config();
    config.features.unsolicited = Feature::Enabled;
    config.features.broadcast = Feature::Enabled;
    config.confirm_timeout = Timeout::from_secs(2).unwrap();

    let db = new_db(&config);
    db.with_database(|db| {
        db.add(0, Some(EventClass::Class1), BinaryInputConfig::default());
        db.add(0, Some(EventClass::Class1), CounterConfig::default());
        db.add(0, Some(EventClass::Class1), AnalogInputConfig::default());
        db.define_attr(
            AttrProp::writable(),
            OwnedAttribute::new(
                AttrSet::Default,
                StringAttr::UserAssignedLocation.variation(),
                OwnedAttrValue::VisibleString("lab".to_string()),
            ),
        )
        .unwrap();
    });

    let called = Arc::new(Mutex::new(BTreeSet::new()));
    let callbacks = ReentrantCallbacks::new(db.clone(), called.clone());
    let mut core = OutstationCore::new(
        &config,
        PhysAddr::None,
        db.clone(),
        Box::new(callbacks.clone()),
        Box::new(callbacks.clone()),
        Box::new(callbacks),
        &clock,
    );

    // 1. NULL unsolicited -> timeout -> second NULL -> confirm.
    assert_eq!(send_expect_fragment(&mut core)[1], 0x82);
    clock.advance(Duration::from_secs(2));
    assert_eq!(send_expect_fragment(&mut core)[1], 0x82);
    run_sync_input(&mut core, input_fragment(&[0xD1, 0x00]));

    // 2. DELAY_MEASURE, WRITE g50v1, COLD_RESTART, WARM_RESTART, IMMEDIATE_FREEZE.
    exchange_ok(&mut core, &[0xC0, 0x17]);
    exchange_ok(
        &mut core,
        &[0xC1, 0x02, 50, 1, 0x07, 0x01, 0, 0, 0, 0, 0, 0],
    );
    exchange_ok(&mut core, &[0xC2, 0x0D]);
    exchange_ok(&mut core, &[0xC3, 0x0E]);
    exchange_ok(&mut core, &[0xC4, 0x07, 20, 0, 0x06]);

    // 3. WRITE analog deadband g34v1, ASSIGN_CLASS, WRITE device attr, WRITE IIN1.7=0.
    exchange_ok(&mut core, &[0xC5, 0x02, 34, 1, 0x00, 0x00, 0x00, 5, 0]);
    exchange_ok(&mut core, &[0xC6, 0x16, 60, 2, 0x06, 1, 0, 0x06]);
    exchange_ok(
        &mut core,
        &[
            0xC7,
            0x02,
            0,
            StringAttr::UserAssignedLocation.variation(),
            0x00,
            0x00,
            0x00,
            0x01,
            2,
            b'o',
            b'k',
        ],
    );
    exchange_ok(&mut core, &[0xC8, 0x02, 80, 1, 0x00, 7, 7, 0]);

    // 4. SELECT + OPERATE g12v1.
    let crob = [12, 1, 0x17, 1, 0, 0x03, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut select_req = vec![0xC9, 0x03];
    select_req.extend_from_slice(&crob);
    exchange_ok(&mut core, &select_req);

    let mut operate_req = vec![0xCA, 0x04];
    operate_req.extend_from_slice(&crob);
    exchange_ok(&mut core, &operate_req);

    // 5. Broadcast RECORD_CURRENT_TIME.
    run_sync_input(
        &mut core,
        broadcast_fragment(&[0xCB, 0x18], BroadcastConfirmMode::NotRequired),
    );

    // 6. Class 1 event -> READ -> wrong seq, unexpected unsol confirm, expected confirm.
    db.transaction(|db| {
        db.update(
            0,
            &BinaryInput::new(true, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });
    exchange_ok(&mut core, &[0xCC, 0x01, 60, 2, 0x06]);
    run_sync_input(&mut core, input_fragment(&[0xCD, 0x00]));
    run_sync_input(&mut core, input_fragment(&[0xDC, 0x00]));
    run_sync_input(&mut core, input_fragment(&[0xCC, 0x00]));

    // 7. Second event -> READ -> interrupt with new request -> READ -> confirm timeout.
    db.transaction(|db| {
        db.update(
            0,
            &BinaryInput::new(false, Flags::ONLINE, Time::synchronized(0)),
            UpdateOptions::default(),
        );
    });
    exchange_ok(&mut core, &[0xCD, 0x01, 60, 2, 0x06]);
    exchange_ok(&mut core, &[0xCE, 0x18]);
    exchange_ok(&mut core, &[0xCF, 0x01, 60, 2, 0x06]);
    clock.advance(Duration::from_secs(2));
    assert!(matches!(core.poll(), Output::Wait(_)));

    let expected: BTreeSet<&'static str> = [
        "get_processing_delay_ms",
        "write_absolute_time",
        "get_application_iin",
        "cold_restart",
        "warm_restart",
        "freeze_counter",
        "support_write_analog_dead_bands",
        "begin_write_analog_dead_bands",
        "write_analog_dead_band",
        "end_write_analog_dead_bands",
        "support_assign_class",
        "begin_assign_class",
        "assign_class",
        "end_assign_class",
        "write_device_attr",
        "begin_confirm",
        "event_cleared",
        "end_confirm",
        "process_request_from_idle",
        "broadcast_received",
        "enter_solicited_confirm_wait",
        "solicited_confirm_timeout",
        "solicited_confirm_received",
        "solicited_confirm_wait_new_request",
        "wrong_solicited_confirm_seq",
        "unexpected_confirm",
        "enter_unsolicited_confirm_wait",
        "unsolicited_confirm_timeout",
        "unsolicited_confirmed",
        "clear_restart_iin",
        "control_begin_fragment",
        "control_select",
        "control_operate",
        "control_end_fragment",
    ]
    .into_iter()
    .collect();

    assert_eq!(*called.lock().unwrap(), expected);
}

// ---------- OutstationTask stalled-write cancellation tests ----------

fn new_stalled_write_task() -> (
    OutstationTask,
    OutstationHandle,
    PhysLayer,
    tokio::io::DuplexStream,
) {
    let mut config = OutstationConfig::new(OUTSTATION, MASTER, EventBufferConfig::all_types(5));
    config.features.unsolicited = Feature::Enabled;
    config.keep_alive_timeout = None;

    let (task, handle) = OutstationTask::create(
        Enabled::Yes,
        LinkModes::test(),
        config,
        PhysAddr::None,
        Box::new(NullApplication),
        Box::new(NullInformation),
        DefaultControlHandler::create(),
    );

    // Duplex buffer smaller than a 17-byte NULL unsolicited link frame so the write stalls.
    let (stream, peer) = tokio::io::duplex(4);
    (task, handle, PhysLayer::duplex(stream), peer)
}

#[tokio::test(start_paused = true)]
async fn stalled_write_is_cancelled_immediately_by_run_until() {
    watchdog(async {
        let (mut task, _handle, mut phys, _peer) = new_stalled_write_task();
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel::<u32>();
        let driver = tokio::spawn(async move {
            let res = task.run_until(&mut phys, cancel_rx).await;
            (task, res)
        });

        // Let the task start writing the NULL unsolicited frame and block on the 4-byte duplex.
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!driver.is_finished());

        // Cancelling unblocks the stalled write immediately without waiting for the peer.
        cancel_tx.send(42).unwrap();
        let (mut task, res) = driver.await.unwrap();
        assert_eq!(res.unwrap(), Ok(42));

        // SessionTeardown reset the task cleanly so a new connection receives a fresh NULL frame.
        let (stream, mut peer) = tokio::io::duplex(64);
        let mut phys2 = PhysLayer::duplex(stream);
        let (cancel_tx2, cancel_rx2) = tokio::sync::oneshot::channel::<()>();
        let driver2 = tokio::spawn(async move { task.run_until(&mut phys2, cancel_rx2).await });

        let mut buf = [0u8; 17];
        tokio::io::AsyncReadExt::read_exact(&mut peer, &mut buf)
            .await
            .unwrap();
        assert_eq!(&buf[..2], &[0x05, 0x64]);
        cancel_tx2.send(()).unwrap();
        assert_eq!(driver2.await.unwrap().unwrap(), Ok(()));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn stalled_write_is_cancelled_immediately_by_shutdown_and_disable() {
    for expected in [StopReason::Shutdown, StopReason::Disable] {
        watchdog(async move {
            let (mut task, handle, mut phys, _peer) = new_stalled_write_task();
            let driver = tokio::spawn(async move { task.run(&mut phys).await });

            tokio::time::sleep(Duration::from_millis(10)).await;
            assert!(!driver.is_finished());

            match expected {
                StopReason::Shutdown => handle.shutdown().await.unwrap(),
                StopReason::Disable => handle.disable().await.unwrap(),
            }

            let err = driver.await.unwrap();
            assert!(
                matches!(err, RunError::Stop(reason) if reason == expected),
                "expected {expected:?}, got {err:?}"
            );
        })
        .await;
    }
}
