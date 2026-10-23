// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use tokio::time::Instant;

use crate::app::Sequence;
use crate::link::EndpointAddress;
use crate::master::association::AssociationConfig;
use crate::master::test_fixtures::null;
use crate::master::tests::harness::create_association;
use crate::master::tests::harness::requests::{class_scan_request, empty_response};
use crate::master::{Classes, ReadRequest, TaskError};
use crate::util::test_support::watchdog;

const COLD_RESTART: u8 = 0x0D;

/// The kinds of task, which each read and filter fragments
#[derive(Copy, Clone, Debug)]
enum Kind {
    NonRead,
    Read,
    LinkStatus,
}

#[tokio::test(start_paused = true)]
async fn response_from_another_outstation_during_any_task_refreshes_its_own_keep_alive() {
    watchdog(async {
        for kind in [Kind::NonRead, Kind::Read, Kind::LinkStatus] {
            // the harness outstation, which sends every response, has a 10 second keep-alive
            let config = AssociationConfig {
                keep_alive_timeout: Some(Duration::from_secs(10)),
                ..AssociationConfig::quiet()
            };
            let mut harness = create_association(config).await;
            let start = Instant::now();

            // a second outstation on the same link
            let mut other = harness
                .master
                .add_association(
                    EndpointAddress::try_new(2048).unwrap(),
                    AssociationConfig::quiet(),
                    null(),
                    null(),
                    null(),
                )
                .await
                .unwrap();

            tokio::time::advance(Duration::from_secs(9)).await;

            // while a request to the second outstation is outstanding, the harness outstation
            // sends a response, which refreshes its keep-alive
            let request = match kind {
                Kind::NonRead => vec![0xC0, COLD_RESTART],
                Kind::Read => class_scan_request(Classes::class0(), Sequence::new(0)),
                Kind::LinkStatus => crate::transport::mock::writer::LINK_STATUS_REQUEST.to_vec(),
            };
            let task = tokio::spawn(async move {
                match kind {
                    Kind::NonRead => other.cold_restart().await.map(|_| ()),
                    Kind::Read => other.read(ReadRequest::ClassScan(Classes::class0())).await,
                    Kind::LinkStatus => other.check_link_status().await,
                }
            });
            harness.expect_write(request).await;
            harness
                .process_response(empty_response(Sequence::new(0)))
                .await;
            // the task ignores the response, which isn't from its outstation
            assert_eq!(
                task.await.unwrap(),
                Err(TaskError::ResponseTimeout),
                "{kind:?}"
            );

            // so a read issued after the original keep-alive deadline runs at once, instead of
            // waiting behind a link status request
            tokio::time::advance(Duration::from_secs(11).saturating_sub(start.elapsed())).await;
            let now = Instant::now();
            let mut association = harness.association.clone();
            let read = tokio::spawn(async move {
                association
                    .read(ReadRequest::ClassScan(Classes::class0()))
                    .await
            });
            harness
                .expect_write_and_respond(
                    class_scan_request(Classes::class0(), Sequence::new(0)),
                    empty_response(Sequence::new(0)),
                )
                .await;
            assert_eq!(read.await.unwrap(), Ok(()), "{kind:?}");
            assert_eq!(now.elapsed(), Duration::ZERO, "{kind:?}");
        }
    })
    .await;
}
