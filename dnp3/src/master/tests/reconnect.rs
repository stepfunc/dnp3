// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! The master's next run after the connection drops or the channel is disabled starts the
//! association over: it redoes the startup sequence, ignores unsolicited data until the integrity
//! poll completes, and doesn't take the first unsolicited response for a repeat of the last one
//! before the run ended

use std::io::ErrorKind;

use crate::app::{FunctionCode, Sequence};
use crate::link::LinkError;
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::requests::{
    disable_unsol_request, empty_response, enable_unsol_request, integrity_poll_request,
    startup_procedure, unsol_confirm, unsol_with_data,
};
use crate::master::tests::harness::{
    create_rerunnable_association, AssocInfoEvent, ReadEvent, TestHarness,
};
use crate::master::{ReadType, TaskType};
use crate::util::lifecycle::{RunError, StopReason};
use crate::util::test_support::watchdog;

/// How the first run ends
#[derive(Copy, Clone, Debug)]
enum End {
    ConnectionDrop,
    Disable,
}

impl End {
    async fn end(self, harness: &mut TestHarness) -> RunError {
        match self {
            End::ConnectionDrop => {
                harness.io.read_error(ErrorKind::ConnectionReset);
                harness.run_again().await
            }
            End::Disable => {
                harness.master.disable().await.unwrap();
                let err = harness.run_again().await;
                harness.master.enable().await.unwrap();
                err
            }
        }
    }

    fn error(self) -> RunError {
        match self {
            End::ConnectionDrop => RunError::Link(LinkError::Stdio(ErrorKind::ConnectionReset)),
            End::Disable => RunError::Stop(StopReason::Disable),
        }
    }
}

/// The start and success of each startup task, the first with sequence number `seq`
fn startup_events(seq: u8) -> Vec<AssocInfoEvent> {
    [
        (
            TaskType::DisableUnsolicited,
            FunctionCode::DisableUnsolicited,
        ),
        (TaskType::StartupIntegrity, FunctionCode::Read),
        (TaskType::EnableUnsolicited, FunctionCode::EnableUnsolicited),
    ]
    .into_iter()
    .zip(seq..)
    .flat_map(|((task, function), seq)| {
        let seq = Sequence::new(seq);
        [
            AssocInfoEvent::TaskStart(task, function, seq),
            AssocInfoEvent::TaskSuccess(task, function, seq),
        ]
    })
    .collect()
}

fn unsolicited_analog() -> Vec<ReadEvent> {
    vec![
        ReadEvent::BeginFragment(ReadType::Unsolicited),
        ReadEvent::AnalogInput,
        ReadEvent::EndFragment(ReadType::Unsolicited),
    ]
}

/// Catches deleting any of the three resets in `Association::reset`: without
/// `auto_tasks.reset()` the second run sends no startup requests; without
/// `startup_integrity_done = false` it confirms and delivers the unsolicited data sent during the
/// integrity poll; without `last_unsol_frag = None` it takes the first unsolicited response for a
/// repeat and doesn't deliver it
async fn next_run_starts_the_association_over(end: End) {
    let mut harness = create_rerunnable_association(AssociationConfig::default()).await;
    let mut seq = Sequence::default();
    startup_procedure(&mut harness, &mut seq).await;
    harness.read_events.pop();
    harness.assoc_events.pop();

    let unsol_seq = Sequence::new(5);
    let last = unsol_with_data(unsol_seq, 42, false);
    harness
        .read_and_expect_write(last.clone(), unsol_confirm(unsol_seq))
        .await;
    assert_eq!(harness.read_events.pop(), unsolicited_analog(), "{end:?}");
    assert_eq!(
        harness.assoc_events.pop(),
        [AssocInfoEvent::Unsolicited(false, unsol_seq)],
        "{end:?}"
    );

    assert_eq!(end.end(&mut harness).await, end.error(), "{end:?}");

    // the startup sequence again, the request sequence numbers continuing from the first run
    harness
        .expect_write_and_respond(disable_unsol_request(seq), empty_response(seq.increment()))
        .await;
    harness.expect_write(integrity_poll_request(seq)).await;
    // unsolicited data before the integrity poll completes is neither confirmed nor delivered: the
    // next write is the ENABLE_UNSOLICITED request
    harness
        .process_response(unsol_with_data(Sequence::new(6), 43, false))
        .await;
    harness
        .process_response(empty_response(seq.increment()))
        .await;
    harness
        .expect_write_and_respond(enable_unsol_request(seq), empty_response(seq.increment()))
        .await;

    assert_eq!(
        harness.read_events.pop(),
        [
            ReadEvent::BeginFragment(ReadType::StartupIntegrity),
            ReadEvent::EndFragment(ReadType::StartupIntegrity),
        ],
        "{end:?}"
    );
    assert_eq!(harness.assoc_events.pop(), startup_events(3), "{end:?}");

    // the same unsolicited response as the last one before the run ended isn't a repeat
    harness
        .read_and_expect_write(last, unsol_confirm(unsol_seq))
        .await;
    assert_eq!(harness.read_events.pop(), unsolicited_analog(), "{end:?}");
    assert_eq!(
        harness.assoc_events.pop(),
        [AssocInfoEvent::Unsolicited(false, unsol_seq)],
        "{end:?}"
    );
    harness.assert_no_events();
}

#[tokio::test]
async fn reconnect_starts_the_association_over() {
    watchdog(next_run_starts_the_association_over(End::ConnectionDrop)).await;
}

#[tokio::test]
async fn reenable_starts_the_association_over() {
    watchdog(next_run_starts_the_association_over(End::Disable)).await;
}
