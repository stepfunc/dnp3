use std::time::Duration;

use crate::app::Sequence;
use crate::master::association::AssociationConfig;
use crate::master::TaskError;

use super::harness::create_association;
use super::harness::requests::startup_procedure;

const COLD_RESTART: u8 = 0x0D;
const WARM_RESTART: u8 = 0x0E;

fn restart_request(function: u8, seq: Sequence) -> Vec<u8> {
    vec![0xC0 | seq.value(), function]
}

fn response(seq: Sequence, objects: &[u8]) -> Vec<u8> {
    let mut data = vec![0xC0 | seq.value(), 0x81, 0x00, 0x00];
    data.extend_from_slice(objects);
    data
}

#[tokio::test]
async fn cold_restart_returns_the_delay_in_seconds() {
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::default()).await;
    startup_procedure(&mut harness, &mut seq).await;

    let mut association = harness.association.clone();
    let restart = tokio::spawn(async move { association.cold_restart().await });

    // g52v1 with a count of one: 2 seconds
    harness
        .expect_write_and_respond(
            restart_request(COLD_RESTART, seq),
            response(seq.increment(), &[52, 1, 0x07, 1, 0x02, 0x00]),
        )
        .await;

    assert_eq!(restart.await.unwrap(), Ok(Duration::from_secs(2)));
}

#[tokio::test]
async fn warm_restart_returns_the_delay_in_milliseconds() {
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::default()).await;
    startup_procedure(&mut harness, &mut seq).await;

    let mut association = harness.association.clone();
    let restart = tokio::spawn(async move { association.warm_restart().await });

    // g52v2 with a count of one: 2 milliseconds
    harness
        .expect_write_and_respond(
            restart_request(WARM_RESTART, seq),
            response(seq.increment(), &[52, 2, 0x07, 1, 0x02, 0x00]),
        )
        .await;

    assert_eq!(restart.await.unwrap(), Ok(Duration::from_millis(2)));
}

#[tokio::test]
async fn restart_fails_when_the_response_has_no_time_delay() {
    let mut seq = Sequence::default();
    let mut harness = create_association(AssociationConfig::default()).await;
    startup_procedure(&mut harness, &mut seq).await;

    let mut association = harness.association.clone();
    let restart = tokio::spawn(async move { association.cold_restart().await });

    // g51v1 (CTO) instead of a time delay
    harness
        .expect_write_and_respond(
            restart_request(COLD_RESTART, seq),
            response(
                seq.increment(),
                &[51, 1, 0x07, 1, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            ),
        )
        .await;

    assert_eq!(
        restart.await.unwrap(),
        Err(TaskError::UnexpectedResponseHeaders)
    );
}
