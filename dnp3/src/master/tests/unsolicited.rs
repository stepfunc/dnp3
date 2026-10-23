// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Unsolicited responses: a valid one is processed and confirmed, a duplicate of the last one
//! accepted is confirmed again without processing, and an invalid one is discarded without
//! processing or confirming it

use crate::app::{ControlField, Iin, Sequence};
use crate::master::association::AssociationConfig;
use crate::master::test_objects::{RELATIVE_TIME_EVENT, SYNCHRONIZED_CTO, TRUNCATED_ANALOG_EVENT};
use crate::master::tests::harness::{AssocInfoEvent, ReadEvent, TestHarness};
use crate::master::ReadType;

use super::harness::create_association;
use super::harness::requests::{
    startup_procedure, unsol_confirm, unsol_null, unsol_null_custom_iin, unsol_with_data,
};
use crate::util::test_support::watchdog;

fn unsol_with_objects(seq: Sequence, objects: &[&[u8]]) -> Vec<u8> {
    let mut response = unsol_null_custom_iin(seq, Iin::default());
    for x in objects {
        response.extend_from_slice(x);
    }
    response
}

fn fragment(events: &[ReadEvent]) -> Vec<ReadEvent> {
    let mut fragment = vec![ReadEvent::BeginFragment(ReadType::Unsolicited)];
    fragment.extend_from_slice(events);
    fragment.push(ReadEvent::EndFragment(ReadType::Unsolicited));
    fragment
}

async fn started() -> TestHarness {
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::default()).await;
    startup_procedure(&mut harness, &mut seq).await;
    harness.read_events.pop();
    harness.assoc_events.pop();
    harness
}

/// Process an unsolicited response that the master must discard, then a valid one. The confirm of
/// the valid one is the next write, so the discarded one wasn't confirmed
async fn expect_discarded(harness: &mut TestHarness, response: Vec<u8>) {
    harness.process_response(response).await;

    let next = Sequence::new(10);
    harness
        .read_and_expect_write(unsol_with_data(next, 42, false), unsol_confirm(next))
        .await;

    assert_eq!(
        harness.read_events.pop(),
        fragment(&[ReadEvent::AnalogInput])
    );
    assert_eq!(
        harness.assoc_events.pop(),
        vec![AssocInfoEvent::Unsolicited(false, next)]
    );
}

#[tokio::test]
async fn malformed_unsolicited_response_is_discarded_without_confirm() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);
        let response = unsol_with_objects(seq, &[TRUNCATED_ANALOG_EVENT]);

        harness.process_response(response.clone()).await;
        // the outstation's retry isn't a duplicate of an accepted response, so it's discarded too
        expect_discarded(&mut harness, response).await;
    })
    .await;
}

#[tokio::test]
async fn unsolicited_relative_time_without_cto_is_discarded_without_confirm() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);
        let response = unsol_with_objects(seq, &[RELATIVE_TIME_EVENT]);

        harness.process_response(response.clone()).await;
        expect_discarded(&mut harness, response).await;
    })
    .await;
}

#[tokio::test]
async fn unsolicited_relative_time_after_cto_is_processed_and_confirmed() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);

        harness
            .read_and_expect_write(
                unsol_with_objects(seq, &[SYNCHRONIZED_CTO, RELATIVE_TIME_EVENT]),
                unsol_confirm(seq),
            )
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment(&[ReadEvent::BinaryInput])
        );
        assert_eq!(
            harness.assoc_events.pop(),
            vec![AssocInfoEvent::Unsolicited(false, seq)]
        );
    })
    .await;
}

#[tokio::test]
async fn duplicate_unsolicited_response_is_confirmed_without_processing() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);

        harness
            .read_and_expect_write(unsol_with_data(seq, 42, false), unsol_confirm(seq))
            .await;
        harness
            .read_and_expect_write(unsol_with_data(seq, 42, false), unsol_confirm(seq))
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment(&[ReadEvent::AnalogInput])
        );
        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::Unsolicited(false, seq),
                AssocInfoEvent::Unsolicited(true, seq)
            ]
        );

        // the same sequence with different data, then the same data with a different sequence, aren't
        // duplicates
        let next = Sequence::new(4);
        for (seq, value) in [(seq, 43), (next, 43)] {
            harness
                .read_and_expect_write(unsol_with_data(seq, value, false), unsol_confirm(seq))
                .await;
            assert_eq!(
                harness.read_events.pop(),
                fragment(&[ReadEvent::AnalogInput]),
                "{seq:?} {value}"
            );
            assert_eq!(
                harness.assoc_events.pop(),
                vec![AssocInfoEvent::Unsolicited(false, seq)],
                "{seq:?} {value}"
            );
        }
    })
    .await;
}

#[tokio::test]
async fn null_unsolicited_response_is_confirmed() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);

        harness
            .read_and_expect_write(unsol_null(seq, false), unsol_confirm(seq))
            .await;

        assert_eq!(harness.read_events.pop(), fragment(&[]));
        assert_eq!(
            harness.assoc_events.pop(),
            vec![AssocInfoEvent::Unsolicited(false, seq)]
        );
    })
    .await;
}

#[tokio::test]
async fn invalid_unsolicited_response_does_not_replace_the_last_accepted_one() {
    watchdog(async {
        let mut harness = started().await;
        let accepted = Sequence::new(3);

        harness
            .read_and_expect_write(
                unsol_with_data(accepted, 42, false),
                unsol_confirm(accepted),
            )
            .await;
        harness
            .process_response(unsol_with_objects(
                Sequence::new(4),
                &[TRUNCATED_ANALOG_EVENT],
            ))
            .await;
        // the next write confirms the duplicate, so the invalid response wasn't confirmed
        harness
            .read_and_expect_write(
                unsol_with_data(accepted, 42, false),
                unsol_confirm(accepted),
            )
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment(&[ReadEvent::AnalogInput])
        );
        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::Unsolicited(false, accepted),
                AssocInfoEvent::Unsolicited(true, accepted)
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn cto_in_a_previous_unsolicited_response_does_not_apply() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);

        harness
            .read_and_expect_write(
                unsol_with_objects(seq, &[SYNCHRONIZED_CTO]),
                unsol_confirm(seq),
            )
            .await;
        assert_eq!(harness.read_events.pop(), fragment(&[]));
        assert_eq!(
            harness.assoc_events.pop(),
            vec![AssocInfoEvent::Unsolicited(false, seq)]
        );

        expect_discarded(
            &mut harness,
            unsol_with_objects(Sequence::new(4), &[RELATIVE_TIME_EVENT]),
        )
        .await;
    })
    .await;
}

#[tokio::test]
async fn valid_measurements_before_relative_time_without_cto_are_not_delivered() {
    watchdog(async {
        let mut harness = started().await;
        let mut response = unsol_with_data(Sequence::new(3), 42, false);
        response.extend_from_slice(RELATIVE_TIME_EVENT);

        expect_discarded(&mut harness, response).await;
    })
    .await;
}

#[tokio::test]
async fn unsolicited_response_without_con_is_processed_without_confirm() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);
        let mut response = unsol_with_data(seq, 42, false);
        let control = ControlField::from(response[0]);
        response[0] = ControlField {
            con: false,
            ..control
        }
        .to_u8();

        harness.process_response(response).await;
        // the next write confirms the next response, so the first one wasn't confirmed
        let next = Sequence::new(4);
        harness
            .read_and_expect_write(unsol_with_data(next, 43, false), unsol_confirm(next))
            .await;

        let mut events = fragment(&[ReadEvent::AnalogInput]);
        events.extend(fragment(&[ReadEvent::AnalogInput]));
        assert_eq!(harness.read_events.pop(), events);
        assert_eq!(
            harness.assoc_events.pop(),
            vec![
                AssocInfoEvent::Unsolicited(false, seq),
                AssocInfoEvent::Unsolicited(false, next)
            ]
        );
    })
    .await;
}

#[tokio::test]
async fn unsolicited_empty_relative_time_headers_without_cto_are_processed_and_confirmed() {
    watchdog(async {
        let mut harness = started().await;
        let seq = Sequence::new(3);
        // g2v3 and g4v3 with a count of 0, with a one-byte then a two-byte count and prefix
        let objects: [&[u8]; 4] = [
            &[0x02, 0x03, 0x17, 0x00],
            &[0x04, 0x03, 0x17, 0x00],
            &[0x02, 0x03, 0x28, 0x00, 0x00],
            &[0x04, 0x03, 0x28, 0x00, 0x00],
        ];

        harness
            .read_and_expect_write(unsol_with_objects(seq, &objects), unsol_confirm(seq))
            .await;

        assert_eq!(
            harness.read_events.pop(),
            fragment(&[
                ReadEvent::BinaryInput,
                ReadEvent::DoubleBitBinaryInput,
                ReadEvent::BinaryInput,
                ReadEvent::DoubleBitBinaryInput,
            ])
        );
        assert_eq!(
            harness.assoc_events.pop(),
            vec![AssocInfoEvent::Unsolicited(false, seq)]
        );
    })
    .await;
}
