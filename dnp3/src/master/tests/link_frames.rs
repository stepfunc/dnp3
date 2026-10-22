//! Tasks over the real link and transport layers: what partial frames, API messages and link-layer
//! messages do to a task's response deadline and outcome

use std::time::Duration;

use scursor::WriteCursor;
use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::app::parse::options::ParseOptions;
use crate::app::{BufferSize, Sequence, Timeout};
use crate::decode::DecodeLevel;
use crate::link::constant::MAX_LINK_FRAME_LENGTH;
use crate::link::format::{format_data_frame, format_header_only, Payload};
use crate::link::header::{ControlField, Header};
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::association::AssociationConfig;
use crate::master::task::MasterTask;
use crate::master::tests::harness::requests::{empty_response, unsol_null};
use crate::master::{
    AssociationHandle, AssociationHandler, AssociationInformation, Classes, MasterChannel,
    MasterChannelConfig, MasterChannelType, ReadHandler, ReadRequest, TaskError,
};
use crate::util::phys::PhysLayer;
use crate::util::session::{Enabled, RunError};
use crate::util::test_support::watchdog;

const MASTER: EndpointAddress = EndpointAddress::raw(1);
const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);
const OTHER: EndpointAddress = EndpointAddress::raw(2048);

const STEP: Duration = Duration::from_millis(250);
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(1);

// link control bytes sent by an outstation (DIR = 0)
const LINK_STATUS: u8 = 0x0B;
const REQUEST_LINK_STATUS: u8 = 0x49;

/// A data frame from an outstation to the master carrying one transport segment
fn segment(source: EndpointAddress, fir: bool, fin: bool, seq: u8, data: &[u8]) -> Vec<u8> {
    let header = Header::unconfirmed_user_data(false, MASTER.wrap(), source.wrap());
    let transport = crate::transport::real::header::Header::new(
        fin,
        fir,
        crate::transport::real::sequence::Sequence::new(seq),
    );
    let mut buffer = [0; MAX_LINK_FRAME_LENGTH];
    let mut cursor = WriteCursor::new(&mut buffer);
    format_data_frame(header, Payload::new(transport.to_u8(), data), &mut cursor)
        .unwrap()
        .frame
        .to_vec()
}

/// A whole fragment from the outstation in one segment
fn fragment(source: EndpointAddress, data: &[u8]) -> Vec<u8> {
    segment(source, true, true, 0, data)
}

fn header_only(source: EndpointAddress, control: u8) -> Vec<u8> {
    let header = Header::new(ControlField::from(control), MASTER.wrap(), source.wrap());
    let mut buffer = [0; MAX_LINK_FRAME_LENGTH];
    let mut cursor = WriteCursor::new(&mut buffer);
    format_header_only(header, &mut cursor)
        .unwrap()
        .frame
        .to_vec()
}

/// The application data of a frame written by the master, which carries a single segment
fn app_data(frame: &[u8]) -> &[u8] {
    // 10 header bytes, the transport header, then the data and its 2-byte CRC
    &frame[11..frame.len().saturating_sub(2)]
}

struct NullHandler;
impl ReadHandler for NullHandler {}
impl AssociationHandler for NullHandler {}
impl AssociationInformation for NullHandler {}

struct Harness {
    task: JoinHandle<RunError>,
    master: MasterChannel,
    association: AssociationHandle,
    io: sfio_tokio_mock_io::Handle,
}

impl Harness {
    async fn new() -> Self {
        let (io, handle) = sfio_tokio_mock_io::mock();
        let mut io = PhysLayer::Mock(io);
        let (tx, rx) = crate::util::channel::request_channel();
        let mut task = MasterTask::new(
            Enabled::Yes,
            LinkModes::stream(LinkErrorMode::Discard),
            ParseOptions::default(),
            MasterChannelConfig {
                master_address: MASTER,
                decode_level: DecodeLevel::nothing(),
                tx_buffer_size: BufferSize::min(),
                rx_buffer_size: BufferSize::min(),
            },
            rx,
        );
        let task = tokio::spawn(async move { task.run(&mut io).await });
        let mut master = MasterChannel::new(tx, MasterChannelType::Stream);
        let association = master
            .add_association(OUTSTATION, config(), null(), null(), null())
            .await
            .unwrap();
        Self {
            task,
            master,
            association,
            io: handle,
        }
    }

    async fn pop_write(&mut self) -> Vec<u8> {
        match self.io.next_event().await {
            sfio_tokio_mock_io::Event::Write(data) => data,
            x => panic!("expected a write, but got: {x:?}"),
        }
    }

    async fn read(&mut self, data: &[u8]) {
        self.io.read(data);
        assert_eq!(self.io.next_event().await, sfio_tokio_mock_io::Event::Read);
    }

    /// Every message sent before this returns has been processed by the master task
    async fn barrier(&mut self) {
        self.master.get_decode_level().await.unwrap();
    }
}

fn config() -> AssociationConfig {
    AssociationConfig {
        response_timeout: Timeout::from_secs(1).unwrap(),
        ..AssociationConfig::quiet()
    }
}

fn null() -> Box<NullHandler> {
    Box::new(NullHandler)
}

/// The kinds of task, which each compute their response deadline
#[derive(Copy, Clone, Debug)]
enum Kind {
    Read,
    NonRead,
    LinkStatus,
}

const KINDS: [Kind; 3] = [Kind::Read, Kind::NonRead, Kind::LinkStatus];

impl Kind {
    /// Start the task, which completes with the outcome and the time it completed
    fn spawn(
        self,
        mut association: AssociationHandle,
    ) -> JoinHandle<(Result<(), TaskError>, Instant)> {
        tokio::spawn(async move {
            let result = match self {
                Kind::Read => {
                    association
                        .read(ReadRequest::ClassScan(Classes::class0()))
                        .await
                }
                Kind::NonRead => association.cold_restart().await.map(|_| ()),
                Kind::LinkStatus => association.check_link_status().await,
            };
            (result, Instant::now())
        })
    }
}

/// Input that arrives while a task waits, without completing a fragment
#[derive(Copy, Clone, Debug)]
enum Trickle {
    /// first segments of a fragment, each one restarting the reassembly
    Segments,
    /// a link frame, one byte at a time
    LinkBytes,
}

/// Every step, the master processes an API message and receives part of a frame. Neither moves
/// the deadline, which is measured from the request.
async fn deadline_holds(kind: Kind, trickle: Trickle) {
    let mut harness = Harness::new().await;
    let task = kind.spawn(harness.association.clone());
    harness.pop_write().await;
    let start = Instant::now();

    let frame = fragment(OUTSTATION, &empty_response(Sequence::new(0)));
    for step in 0..8u8 {
        match trickle {
            Trickle::Segments => {
                let data = empty_response(Sequence::new(0));
                harness
                    .read(&segment(OUTSTATION, true, false, step, &data))
                    .await
            }
            Trickle::LinkBytes => {
                let index = usize::from(step);
                harness.read(&frame[index..index + 1]).await
            }
        }
        harness.barrier().await;
        tokio::time::advance(STEP).await;
    }

    let (result, completed) = task.await.unwrap();
    assert_eq!(
        result,
        Err(TaskError::ResponseTimeout),
        "{kind:?} {trickle:?}"
    );
    let elapsed = completed.duration_since(start);
    assert!(
        elapsed >= RESPONSE_TIMEOUT && elapsed <= RESPONSE_TIMEOUT.saturating_add(STEP),
        "{kind:?} {trickle:?}: {elapsed:?}"
    );
    assert!(!harness.task.is_finished());
}

#[tokio::test(start_paused = true)]
async fn api_messages_and_partial_segments_do_not_extend_the_response_deadline() {
    watchdog(async {
        for kind in KINDS {
            deadline_holds(kind, Trickle::Segments).await;
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn api_messages_and_partial_link_frames_do_not_extend_the_response_deadline() {
    watchdog(async {
        for kind in KINDS {
            deadline_holds(kind, Trickle::LinkBytes).await;
        }
    })
    .await;
}

/// Start a link status request, returning the frame the master wrote
async fn start_link_status(
    harness: &mut Harness,
) -> (Vec<u8>, JoinHandle<(Result<(), TaskError>, Instant)>) {
    let task = Kind::LinkStatus.spawn(harness.association.clone());
    let request = harness.pop_write().await;
    (request, task)
}

#[tokio::test(start_paused = true)]
async fn link_status_request_succeeds_on_a_link_status_response() {
    watchdog(async {
        let mut harness = Harness::new().await;
        let (request, task) = start_link_status(&mut harness).await;
        // REQUEST_LINK_STATUS from the master to the outstation
        assert_eq!(request[3..8], [0xC9, 0x00, 0x04, 0x01, 0x00]);
        harness.read(&header_only(OUTSTATION, LINK_STATUS)).await;
        assert_eq!(task.await.unwrap().0, Ok(()));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn link_status_request_succeeds_on_any_link_layer_message() {
    watchdog(async {
        // from another outstation
        let mut harness = Harness::new().await;
        let (_, task) = start_link_status(&mut harness).await;
        harness.read(&header_only(OTHER, LINK_STATUS)).await;
        assert_eq!(task.await.unwrap().0, Ok(()));

        // or a link status request from the outstation, which the link layer answers
        let (_, task) = start_link_status(&mut harness).await;
        harness
            .read(&header_only(OUTSTATION, REQUEST_LINK_STATUS))
            .await;
        let reply = harness.pop_write().await;
        assert_eq!(reply[3..8], [0x8B, 0x00, 0x04, 0x01, 0x00]);
        assert_eq!(task.await.unwrap().0, Ok(()));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn link_status_request_fails_on_a_fragment() {
    watchdog(async {
        let mut harness = Harness::new().await;

        // a solicited response
        let (_, task) = start_link_status(&mut harness).await;
        harness
            .read(&fragment(OUTSTATION, &empty_response(Sequence::new(0))))
            .await;
        assert_eq!(
            task.await.unwrap().0,
            Err(TaskError::UnexpectedResponseHeaders)
        );

        // a fragment too short to be a response
        let (_, task) = start_link_status(&mut harness).await;
        harness
            .read(&fragment(OUTSTATION, &[0xC0, 0x81, 0x00]))
            .await;
        assert_eq!(
            task.await.unwrap().0,
            Err(TaskError::UnexpectedResponseHeaders)
        );

        // an unsolicited response, which is still processed and confirmed
        let (_, task) = start_link_status(&mut harness).await;
        harness
            .read(&fragment(OUTSTATION, &unsol_null(Sequence::new(3), false)))
            .await;
        let confirm = harness.pop_write().await;
        assert_eq!(app_data(&confirm), [0xD3, 0x00]);
        assert_eq!(
            task.await.unwrap().0,
            Err(TaskError::UnexpectedResponseHeaders)
        );
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn application_tasks_ignore_link_layer_messages() {
    watchdog(async {
        for kind in [Kind::Read, Kind::NonRead] {
            let mut harness = Harness::new().await;
            let task = kind.spawn(harness.association.clone());
            harness.pop_write().await;

            harness.read(&header_only(OUTSTATION, LINK_STATUS)).await;
            harness
                .read(&header_only(OUTSTATION, REQUEST_LINK_STATUS))
                .await;
            // the link layer answers the request
            let reply = harness.pop_write().await;
            assert_eq!(reply[3..8], [0x8B, 0x00, 0x04, 0x01, 0x00], "{kind:?}");
            harness.barrier().await;
            assert!(!task.is_finished(), "{kind:?}");

            let response = match kind {
                Kind::NonRead => vec![0xC0, 0x81, 0x00, 0x00, 52, 1, 0x07, 1, 0x02, 0x00],
                _ => empty_response(Sequence::new(0)),
            };
            harness.read(&fragment(OUTSTATION, &response)).await;
            assert_eq!(task.await.unwrap().0, Ok(()), "{kind:?}");
        }
    })
    .await;
}
