use std::time::Duration;

use tokio::time::Instant;

use crate::master::association::AssociationConfig;
use crate::master::TaskError;

use super::harness::create_association;

const STEP: Duration = Duration::from_millis(250);

#[tokio::test]
async fn link_status_times_out_while_api_messages_keep_arriving() {
    tokio::time::pause();
    let mut harness = create_association(AssociationConfig::quiet()).await;
    let start = Instant::now();

    let mut association = harness.association.clone();
    let check = tokio::spawn(async move {
        let result = association.check_link_status().await;
        (result, Instant::now())
    });

    // once both have returned, the master has received the link status request
    harness.master.get_decode_level().await.unwrap();
    harness.master.get_decode_level().await.unwrap();

    // the outstation never answers, but the master keeps receiving API messages
    for _ in 0..10 {
        tokio::time::advance(STEP).await;
        harness.master.get_decode_level().await.unwrap();
    }

    let (result, completed) = check.await.unwrap();
    assert_eq!(result, Err(TaskError::ResponseTimeout));
    // the 1 second response timeout is measured from the request, and the master
    // notices it no later than the next step
    let elapsed = completed - start;
    assert!(elapsed >= Duration::from_secs(1), "{elapsed:?}");
    assert!(elapsed <= Duration::from_secs(1) + STEP, "{elapsed:?}");
}
