// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use std::time::Duration;

use crate::app::{ObjectParseError, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::TaskError;

use super::harness::create_association;
use crate::util::test_support::watchdog;

#[derive(Copy, Clone, Debug)]
enum Restart {
    Cold,
    Warm,
}

impl Restart {
    fn function(self) -> u8 {
        match self {
            Restart::Cold => 0x0D,
            Restart::Warm => 0x0E,
        }
    }
}

fn response(seq: Sequence, objects: &[u8]) -> Vec<u8> {
    let mut data = vec![0xC0 | seq.value(), 0x81, 0x00, 0x00];
    data.extend_from_slice(objects);
    data
}

/// A restart request, the objects of the response to it, and the result
struct Case {
    restart: Restart,
    objects: &'static [u8],
    expected: Result<Duration, TaskError>,
}

const CASES: [Case; 14] = [
    // g52v1 with a count of one: 2 seconds
    Case {
        restart: Restart::Cold,
        objects: &[52, 1, 0x07, 1, 0x02, 0x00],
        expected: Ok(Duration::from_secs(2)),
    },
    // g52v2 with a count of one: 2 milliseconds
    Case {
        restart: Restart::Warm,
        objects: &[52, 2, 0x07, 1, 0x02, 0x00],
        expected: Ok(Duration::from_millis(2)),
    },
    // g52v1 with a 2-byte count of one: 3 seconds
    Case {
        restart: Restart::Cold,
        objects: &[52, 1, 0x08, 1, 0, 0x03, 0x00],
        expected: Ok(Duration::from_secs(3)),
    },
    // g51v1 (CTO) instead of a time delay
    Case {
        restart: Restart::Cold,
        objects: &[51, 1, 0x07, 1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // g1v2 with a 1-byte start/stop instead of a counted time delay
    Case {
        restart: Restart::Cold,
        objects: &[1, 2, 0x00, 0, 0, 0x01],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // g52v1 with a count of zero
    Case {
        restart: Restart::Cold,
        objects: &[52, 1, 0x07, 0],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // g52v1 with a count of two
    Case {
        restart: Restart::Cold,
        objects: &[52, 1, 0x07, 2, 0x02, 0x00, 0x03, 0x00],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // g52v2 with a count of zero
    Case {
        restart: Restart::Warm,
        objects: &[52, 2, 0x07, 0],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // g52v2 with a count of two
    Case {
        restart: Restart::Warm,
        objects: &[52, 2, 0x07, 2, 0x02, 0x00, 0x03, 0x00],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // two g52v2 headers, each with a count of one
    Case {
        restart: Restart::Warm,
        objects: &[52, 2, 0x07, 1, 0x02, 0x00, 52, 2, 0x07, 1, 0x03, 0x00],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // no object header
    Case {
        restart: Restart::Cold,
        objects: &[],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // two g52v1 time delays
    Case {
        restart: Restart::Cold,
        objects: &[52, 1, 0x07, 1, 0x02, 0x00, 52, 1, 0x07, 1, 0x03, 0x00],
        expected: Err(TaskError::UnexpectedResponseHeaders),
    },
    // g52v1 cut short after its qualifier
    Case {
        restart: Restart::Cold,
        objects: &[52, 1, 0x07],
        expected: Err(TaskError::MalformedResponse(
            ObjectParseError::InsufficientBytes,
        )),
    },
    // a variation of g52 that doesn't exist
    Case {
        restart: Restart::Warm,
        objects: &[52, 9, 0x07, 1, 0x02, 0x00],
        expected: Err(TaskError::MalformedResponse(
            ObjectParseError::UnknownGroupVariation(52, 9),
        )),
    },
];

#[tokio::test]
async fn restart_returns_the_time_delay_of_the_response() {
    watchdog(async {
        for case in CASES {
            let mut harness = create_association(AssociationConfig::quiet()).await;
            let mut association = harness.association.clone();
            let restart = tokio::spawn(async move {
                match case.restart {
                    Restart::Cold => association.cold_restart().await,
                    Restart::Warm => association.warm_restart().await,
                }
            });

            let seq = Sequence::default();
            harness
                .expect_write_and_respond(
                    vec![0xC0, case.restart.function()],
                    response(seq, case.objects),
                )
                .await;

            assert_eq!(
                restart.await.unwrap(),
                case.expected,
                "{:?} {:?}",
                case.restart,
                case.objects
            );

            // nothing else is sent once the master has processed the response
            harness.barrier().await;
            harness.assert_no_events();
        }
    })
    .await;
}
