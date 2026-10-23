// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! How the master schedules the requests, automatic tasks, polls and keep-alives of its
//! associations, over the real link and transport layers with a paused clock
//!
//! The rules these tests establish:
//! - User requests (reads, commands and other API calls) of every association go before the
//!   automatic tasks, polls and keep-alives of any association. Each association runs its own user
//!   requests in the order they were queued.
//! - Associations take turns. The association that just started a task goes to the back of the
//!   line, both among user requests and among automatic tasks, polls and keep-alives, so one busy
//!   association doesn't hold up the others.
//! - Within an association, automatic tasks (such as the startup integrity scan) go before polls,
//!   and a due poll goes before a due keep-alive.
//! - A poll first runs one period after it's added, and then one period after each run
//!   completes, so a slow response delays the next run by as much. A failed run, whether it timed
//!   out or was rejected, is rescheduled the same way and isn't retried sooner, unless the poll was
//!   demanded while it ran.
//! - Demanding a poll runs it at once, and its next run is one period after that run completes.
//!   Demanding a poll while it runs makes it run again as soon as that run completes or fails. A
//!   removed poll never runs again.
//! - Within an association, when the master picks a poll it takes the first one added that is
//!   already due, whichever came due first. If no poll is due, it waits for the earliest deadline.
//! - With a keep-alive timeout, the master sends `REQUEST_LINK_STATUS` once nothing has been
//!   received from the outstation for that long, and not before.
//! - The period restarts whenever the master receives a parsed response (solicited or
//!   unsolicited) or a link-layer message (a `LINK_STATUS` reply or a link-layer request) from the
//!   outstation. A malformed fragment, or a transport segment that doesn't complete a fragment,
//!   doesn't restart it.
//! - An unanswered keep-alive is sent again after each response timeout, until it's answered.
//! - Between tasks the master sleeps until the next one is due. It doesn't spin, so the paused
//!   clock keeps advancing; a test that would spin fails on the watchdog.

use std::time::Duration;

use tokio::task::JoinHandle;
use tokio::time::Instant;

use crate::app::{BufferSize, FunctionCode, Iin, Iin1, Iin2, Sequence, Timeout};
use crate::decode::DecodeLevel;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::master::association::AssociationConfig;
use crate::master::task::MasterTask;
use crate::master::tests::harness::requests::{
    class_scan_request, empty_response, empty_response_custom_iin, unsol_null,
};
use crate::master::tests::harness::{barrier, expect_idle_for};
use crate::master::tests::link_frames::{
    app_data, fragment, header_only, null, segment, LINK_STATUS, MASTER, OTHER, OUTSTATION,
    REQUEST_LINK_STATUS,
};
use crate::master::{
    AssociationHandle, Classes, EventClasses, MasterChannel, MasterChannelConfig,
    MasterChannelType, PollHandle, ReadRequest, TaskError,
};
use crate::util::phys::PhysLayer;
use crate::util::session::Enabled;
use crate::util::test_support::watchdog;

/// Two outstations on one link
const A: EndpointAddress = OUTSTATION;
const B: EndpointAddress = OTHER;

/// Longer than any test waits for a write, so a write that never comes fails the test
const NO_WRITE_WITHIN: Duration = Duration::from_secs(3600);

// link control bytes sent by the master (DIR = 1, PRM = 1)
const MASTER_USER_DATA: u8 = 0xC4;
const MASTER_REQUEST_LINK_STATUS: u8 = 0xC9;

const fn secs(secs: u64) -> Duration {
    Duration::from_secs(secs)
}

const fn millis(millis: u64) -> Duration {
    Duration::from_millis(millis)
}

/// A frame the master wrote
#[derive(Clone, Debug, PartialEq)]
enum Sent {
    /// An application request to an outstation: its function code and object headers, without
    /// the control byte
    Request(EndpointAddress, Vec<u8>),
    /// `REQUEST_LINK_STATUS` to an outstation
    LinkStatus(EndpointAddress),
}

/// One event class (1-3), or class 0 for 0
fn class(class: u8) -> Classes {
    Classes::new(
        class == 0,
        EventClasses::new(class == 1, class == 2, class == 3),
    )
}

/// A read of `classes` sent to `destination`
fn read(destination: EndpointAddress, classes: Classes) -> Sent {
    let request = class_scan_request(classes, Sequence::new(0));
    Sent::Request(destination, request[1..].to_vec())
}

/// A confirm sent to `destination`
fn confirm(destination: EndpointAddress) -> Sent {
    Sent::Request(destination, vec![FunctionCode::Confirm.as_u8()])
}

/// An association config with a response timeout and nothing to do on startup
fn quiet(response_timeout: u64) -> AssociationConfig {
    AssociationConfig {
        response_timeout: Timeout::from_secs(response_timeout).unwrap(),
        ..AssociationConfig::quiet()
    }
}

/// [`quiet`] with a keep-alive timeout
fn keep_alive(response_timeout: u64, keep_alive_timeout: Duration) -> AssociationConfig {
    AssociationConfig {
        keep_alive_timeout: Some(keep_alive_timeout),
        ..quiet(response_timeout)
    }
}

/// A master with no associations, and the clock it's measured against
struct Harness {
    master: MasterChannel,
    io: sfio_tokio_mock_io::Handle,
    start: Instant,
    /// The destination and sequence of the last request the master wrote
    last: Option<(EndpointAddress, Sequence)>,
}

impl Harness {
    fn new() -> Self {
        let (io, handle) = sfio_tokio_mock_io::mock();
        let mut io = PhysLayer::mock(io);
        let (tx, rx) = crate::util::channel::request_channel();
        let mut task = MasterTask::new(
            Enabled::Yes,
            LinkModes::stream(LinkErrorMode::Discard),
            MasterChannelConfig {
                master_address: MASTER,
                decode_level: DecodeLevel::nothing(),
                tx_buffer_size: BufferSize::min(),
                rx_buffer_size: BufferSize::min(),
            },
            rx,
        );
        tokio::spawn(async move { task.run(&mut io).await });
        Self {
            master: MasterChannel::new(tx, MasterChannelType::Stream),
            io: handle,
            start: Instant::now(),
            last: None,
        }
    }

    async fn add(
        &mut self,
        address: EndpointAddress,
        config: AssociationConfig,
    ) -> AssociationHandle {
        self.master
            .add_association(address, config, null(), null(), null())
            .await
            .unwrap()
    }

    /// The next frame the master writes, and when it writes it
    async fn next(&mut self) -> (Duration, Sent) {
        let event = tokio::time::timeout(NO_WRITE_WITHIN, self.io.next_event())
            .await
            .expect("the master didn't write");
        let sfio_tokio_mock_io::Event::Write(frame) = event else {
            panic!("expected a write, but got: {event:?}");
        };
        let destination = EndpointAddress::raw(u16::from_le_bytes([frame[4], frame[5]]));
        let sent = match frame[3] {
            MASTER_REQUEST_LINK_STATUS => Sent::LinkStatus(destination),
            MASTER_USER_DATA => {
                let data = app_data(&frame);
                self.last = Some((destination, Sequence::new(data[0])));
                Sent::Request(destination, data[1..].to_vec())
            }
            control => panic!("unexpected link control: {control:#04X}"),
        };
        (self.start.elapsed(), sent)
    }

    async fn receive(&mut self, frame: &[u8]) {
        self.io.read(frame);
        assert_eq!(self.io.next_event().await, sfio_tokio_mock_io::Event::Read);
    }

    /// Answer the last request with an empty response
    async fn respond(&mut self) {
        self.respond_with_iin(Iin::default()).await;
    }

    /// Answer the last request with an empty response carrying `iin`
    async fn respond_with_iin(&mut self, iin: Iin) {
        let (source, seq) = self.last.take().expect("no request to answer");
        self.receive(&fragment(source, &empty_response_custom_iin(seq, iin)))
            .await;
    }

    /// The outstation at `source` answers a link status request
    async fn reply_link_status(&mut self, source: EndpointAddress) {
        self.receive(&header_only(source, LINK_STATUS)).await;
    }

    /// Wait until the master has processed the messages of the tasks spawned so far.
    ///
    /// On the single-threaded test runtime, the spawned tasks run and send their messages while
    /// the first round trip waits, so they sit ahead of the second.
    async fn settle(&mut self) {
        for _ in 0..2 {
            barrier(&mut self.master).await;
        }
    }
}

fn spawn_read(
    association: &AssociationHandle,
    classes: Classes,
) -> JoinHandle<Result<(), TaskError>> {
    let mut association = association.clone();
    tokio::spawn(async move { association.read(ReadRequest::ClassScan(classes)).await })
}

async fn add_poll(
    association: &mut AssociationHandle,
    classes: Classes,
    period: Duration,
) -> PollHandle {
    association
        .add_poll(ReadRequest::ClassScan(classes), period)
        .await
        .unwrap()
}

/// Expect the next writes, answering each request at once
async fn expect_answered(harness: &mut Harness, expected: &[(Duration, Sent)]) {
    for expected in expected {
        let next = harness.next().await;
        assert_eq!(&next, expected);
        match next.1 {
            Sent::Request(..) => harness.respond().await,
            Sent::LinkStatus(destination) => harness.reply_link_status(destination).await,
        }
    }
}

// --- associations take turns ---

#[tokio::test(start_paused = true)]
async fn queued_user_requests_alternate_between_associations() {
    watchdog(async {
        let mut harness = Harness::new();
        let a = harness.add(A, quiet(1)).await;
        let b = harness.add(B, quiet(1)).await;

        // the master is busy with a read of A, while three more reads of A and two of B queue up
        let first = spawn_read(&a, class(0));
        assert_eq!(harness.next().await, (secs(0), read(A, class(0))));
        let mut reads = vec![first];
        for (association, n) in [(&a, 1), (&a, 2), (&a, 3), (&b, 1), (&b, 2)] {
            reads.push(spawn_read(association, class(n)));
        }
        harness.settle().await;
        harness.respond().await;

        expect_answered(
            &mut harness,
            &[
                (secs(0), read(B, class(1))),
                (secs(0), read(A, class(1))),
                (secs(0), read(B, class(2))),
                (secs(0), read(A, class(2))),
                (secs(0), read(A, class(3))),
            ],
        )
        .await;
        for read in reads {
            assert_eq!(read.await.unwrap(), Ok(()));
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn user_requests_of_every_association_go_before_auto_tasks_and_polls() {
    watchdog(async {
        let integrity = AssociationConfig {
            startup_integrity_classes: class(0),
            ..quiet(60)
        };
        let mut harness = Harness::new();
        let mut a = harness.add(A, integrity).await;
        assert_eq!(harness.next().await, (secs(0), read(A, class(0))));

        // while A's startup integrity scan waits for its response: B is added, which has its own
        // startup integrity scan to do, each association gets a poll and a queued read, and the
        // polls fall due
        let mut b = harness.add(B, integrity).await;
        let _poll_a = add_poll(&mut a, class(1), secs(5)).await;
        let _poll_b = add_poll(&mut b, class(2), secs(5)).await;
        let reads = [spawn_read(&a, class(3)), spawn_read(&b, class(3))];
        harness.settle().await;
        tokio::time::advance(secs(5)).await;
        harness.respond().await;

        expect_answered(
            &mut harness,
            &[
                // the user requests, A first because B was added behind it
                (secs(5), read(A, class(3))),
                (secs(5), read(B, class(3))),
                // then A's poll and B's startup integrity scan, which goes before B's poll
                (secs(5), read(A, class(1))),
                (secs(5), read(B, class(0))),
                (secs(5), read(B, class(2))),
                // the next round of polls
                (secs(10), read(A, class(1))),
                (secs(10), read(B, class(2))),
            ],
        )
        .await;
        for read in reads {
            assert_eq!(read.await.unwrap(), Ok(()));
        }
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn due_polls_alternate_between_associations_every_round() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(1)).await;
        let mut b = harness.add(B, quiet(1)).await;
        let _polls = [
            add_poll(&mut a, class(1), secs(10)).await,
            add_poll(&mut a, class(2), secs(10)).await,
            add_poll(&mut b, class(1), secs(10)).await,
            add_poll(&mut b, class(2), secs(10)).await,
        ];

        for round in [secs(10), secs(20), secs(30)] {
            expect_answered(
                &mut harness,
                &[
                    (round, read(A, class(1))),
                    (round, read(B, class(1))),
                    (round, read(A, class(2))),
                    (round, read(B, class(2))),
                ],
            )
            .await;
        }
    })
    .await;
}

// --- polls ---

#[tokio::test(start_paused = true)]
async fn a_poll_runs_one_period_after_it_is_added_and_one_period_after_each_run_completes() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(5)).await;
        let _poll = add_poll(&mut a, class(1), secs(10)).await;

        expect_answered(&mut harness, &[(secs(10), read(A, class(1)))]).await;

        // the outstation takes 3 seconds to answer, which pushes the next run back as much
        assert_eq!(harness.next().await, (secs(20), read(A, class(1))));
        tokio::time::advance(secs(3)).await;
        harness.respond().await;

        expect_answered(&mut harness, &[(secs(33), read(A, class(1)))]).await;
        assert_eq!(harness.next().await, (secs(43), read(A, class(1))));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_failed_poll_runs_again_one_period_after_it_fails() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(1)).await;
        let _poll = add_poll(&mut a, class(1), secs(10)).await;

        // no response: the poll fails when the response times out
        assert_eq!(harness.next().await, (secs(10), read(A, class(1))));

        // a response with IIN2.1 (object unknown) and no data: the poll fails at once
        assert_eq!(harness.next().await, (secs(21), read(A, class(1))));
        harness
            .respond_with_iin(Iin::new(Iin1::default(), Iin2::new(0x02)))
            .await;

        assert_eq!(harness.next().await, (secs(31), read(A, class(1))));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn demanding_a_poll_runs_it_at_once_and_restarts_its_period() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(1)).await;
        let mut poll = add_poll(&mut a, class(1), secs(10)).await;

        tokio::time::advance(secs(3)).await;
        poll.demand().await.unwrap();
        expect_answered(
            &mut harness,
            &[(secs(3), read(A, class(1))), (secs(13), read(A, class(1)))],
        )
        .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn demanding_a_poll_while_it_runs_runs_it_again_when_that_run_completes() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(5)).await;
        let mut poll = add_poll(&mut a, class(1), secs(10)).await;

        // the poll is demanded while it waits for its response, which comes 2 seconds later
        assert_eq!(harness.next().await, (secs(10), read(A, class(1))));
        poll.demand().await.unwrap();
        harness.settle().await;
        tokio::time::advance(secs(2)).await;
        harness.respond().await;

        expect_answered(
            &mut harness,
            &[(secs(12), read(A, class(1))), (secs(22), read(A, class(1)))],
        )
        .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn demanding_a_poll_whose_run_then_times_out_runs_it_again_when_that_run_fails() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(5)).await;
        let mut poll = add_poll(&mut a, class(1), secs(10)).await;

        // the poll is demanded while it waits for a response that never comes
        assert_eq!(harness.next().await, (secs(10), read(A, class(1))));
        poll.demand().await.unwrap();
        harness.settle().await;

        expect_answered(
            &mut harness,
            &[(secs(15), read(A, class(1))), (secs(25), read(A, class(1)))],
        )
        .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_removed_poll_never_runs_again() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(1)).await;
        let poll = add_poll(&mut a, class(1), secs(10)).await;

        expect_answered(&mut harness, &[(secs(10), read(A, class(1)))]).await;
        tokio::time::advance(secs(2)).await;
        poll.remove().await.unwrap();
        expect_idle_for(&mut harness.io, secs(100)).await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn the_poll_that_comes_due_first_runs_first_and_polls_due_together_run_in_the_order_added() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(1)).await;
        let _polls = [
            add_poll(&mut a, class(1), secs(10)).await,
            add_poll(&mut a, class(2), secs(4)).await,
        ];

        expect_answered(
            &mut harness,
            &[
                (secs(4), read(A, class(2))),
                (secs(8), read(A, class(2))),
                (secs(10), read(A, class(1))),
                (secs(12), read(A, class(2))),
                (secs(16), read(A, class(2))),
                // both are due
                (secs(20), read(A, class(1))),
                (secs(20), read(A, class(2))),
            ],
        )
        .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn polls_already_due_when_the_master_becomes_free_run_in_the_order_added() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, quiet(10)).await;
        let _polls = [
            add_poll(&mut a, class(1), secs(10)).await,
            add_poll(&mut a, class(2), secs(4)).await,
        ];

        // a user read keeps the master busy from 3 s to 12 s, while both polls come due: the
        // second at 4 s, the first at 10 s
        tokio::time::advance(secs(3)).await;
        let user_read = spawn_read(&a, class(0));
        assert_eq!(harness.next().await, (secs(3), read(A, class(0))));
        tokio::time::advance(secs(9)).await;
        harness.respond().await;
        assert_eq!(user_read.await.unwrap(), Ok(()));

        expect_answered(
            &mut harness,
            &[
                (secs(12), read(A, class(1))),
                (secs(12), read(A, class(2))),
                (secs(16), read(A, class(2))),
                (secs(20), read(A, class(2))),
                (secs(22), read(A, class(1))),
            ],
        )
        .await;
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn a_due_poll_goes_before_a_due_keep_alive() {
    watchdog(async {
        let mut harness = Harness::new();
        let mut a = harness.add(A, keep_alive(1, secs(10))).await;
        let _poll = add_poll(&mut a, class(1), secs(10)).await;

        // both are due, and the poll goes first. It isn't answered, so the keep-alive follows as
        // soon as the poll times out
        assert_eq!(harness.next().await, (secs(10), read(A, class(1))));
        assert_eq!(harness.next().await, (secs(11), Sent::LinkStatus(A)));
        harness.reply_link_status(A).await;

        // both are due again. While the outstation answers the polls, the keep-alive is never
        // needed
        expect_answered(
            &mut harness,
            &[(secs(21), read(A, class(1))), (secs(31), read(A, class(1)))],
        )
        .await;
        assert_eq!(harness.next().await, (secs(41), read(A, class(1))));
    })
    .await;
}

// --- keep-alive ---

#[tokio::test(start_paused = true)]
async fn keep_alive_requests_link_status_after_the_link_is_silent_for_the_timeout() {
    watchdog(async {
        let mut harness = Harness::new();
        harness.add(A, keep_alive(1, secs(10))).await;

        expect_answered(&mut harness, &[(secs(10), Sent::LinkStatus(A))]).await;

        // the reply comes 500 ms late, and the period restarts from the reply
        assert_eq!(harness.next().await, (secs(20), Sent::LinkStatus(A)));
        tokio::time::advance(millis(500)).await;
        harness.reply_link_status(A).await;

        assert_eq!(harness.next().await, (millis(30_500), Sent::LinkStatus(A)));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn responses_and_link_layer_messages_from_the_outstation_restart_the_keep_alive_period() {
    watchdog(async {
        let mut harness = Harness::new();
        let a = harness.add(A, keep_alive(1, secs(10))).await;

        // an unsolicited response, which the master confirms
        tokio::time::advance(secs(7)).await;
        harness
            .receive(&fragment(A, &unsol_null(Sequence::new(0), false)))
            .await;
        assert_eq!(harness.next().await, (secs(7), confirm(A)));
        expect_answered(&mut harness, &[(secs(17), Sent::LinkStatus(A))]).await;

        // the response to a user read
        tokio::time::advance(secs(3)).await;
        let user_read = spawn_read(&a, class(0));
        expect_answered(&mut harness, &[(secs(20), read(A, class(0)))]).await;
        assert_eq!(user_read.await.unwrap(), Ok(()));
        expect_answered(&mut harness, &[(secs(30), Sent::LinkStatus(A))]).await;

        // a link-layer request, which the link layer answers
        tokio::time::advance(secs(4)).await;
        harness.receive(&header_only(A, REQUEST_LINK_STATUS)).await;
        let reply = match harness.io.next_event().await {
            sfio_tokio_mock_io::Event::Write(frame) => frame,
            x => panic!("expected the link status reply, but got: {x:?}"),
        };
        assert_eq!(reply[3..8], [0x8B, 0x00, 0x04, 0x01, 0x00]);
        expect_answered(&mut harness, &[(secs(44), Sent::LinkStatus(A))]).await;

        // a malformed fragment doesn't count
        tokio::time::advance(secs(5)).await;
        harness.receive(&fragment(A, &[0xC0, 0x81, 0x00])).await;
        expect_answered(&mut harness, &[(secs(54), Sent::LinkStatus(A))]).await;

        // nor does a first segment that doesn't complete a fragment
        tokio::time::advance(secs(5)).await;
        let data = empty_response(Sequence::new(0));
        harness.receive(&segment(A, true, false, 0, &data)).await;
        assert_eq!(harness.next().await, (secs(64), Sent::LinkStatus(A)));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn an_unanswered_keep_alive_is_repeated_after_each_response_timeout() {
    watchdog(async {
        let mut harness = Harness::new();
        harness.add(A, keep_alive(1, secs(10))).await;

        assert_eq!(harness.next().await, (secs(10), Sent::LinkStatus(A)));
        assert_eq!(harness.next().await, (secs(11), Sent::LinkStatus(A)));
        expect_answered(&mut harness, &[(secs(12), Sent::LinkStatus(A))]).await;
        assert_eq!(harness.next().await, (secs(22), Sent::LinkStatus(A)));
    })
    .await;
}

#[tokio::test(start_paused = true)]
async fn each_association_has_its_own_keep_alive() {
    watchdog(async {
        let mut harness = Harness::new();
        harness.add(A, keep_alive(1, secs(10))).await;
        tokio::time::advance(secs(4)).await;
        harness.add(B, keep_alive(1, secs(3))).await;

        expect_answered(
            &mut harness,
            &[
                (secs(7), Sent::LinkStatus(B)),
                (secs(10), Sent::LinkStatus(A)),
                (secs(10), Sent::LinkStatus(B)),
                (secs(13), Sent::LinkStatus(B)),
            ],
        )
        .await;
    })
    .await;
}
