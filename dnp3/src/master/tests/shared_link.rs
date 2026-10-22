use std::time::Duration;

use tokio::time::Instant;

use crate::app::Sequence;
use crate::link::EndpointAddress;
use crate::master::association::AssociationConfig;
use crate::master::tests::harness::create_association;
use crate::master::tests::harness::requests::{class_scan_request, empty_response};
use crate::master::{
    AssociationHandler, AssociationInformation, Classes, ReadHandler, ReadRequest, TaskError,
};

struct NullHandler;
impl ReadHandler for NullHandler {}
impl AssociationHandler for NullHandler {}
impl AssociationInformation for NullHandler {}

const COLD_RESTART: u8 = 0x0D;

#[tokio::test]
async fn response_from_another_outstation_refreshes_its_own_keep_alive() {
    tokio::time::pause();

    // the harness outstation, which sends every response, has a 10 second keep-alive
    let config = AssociationConfig {
        keep_alive_timeout: Some(Duration::from_secs(10)),
        ..AssociationConfig::quiet()
    };
    let mut harness = create_association(config).await;

    // a second outstation on the same link
    let mut other = harness
        .master
        .add_association(
            EndpointAddress::try_new(2048).unwrap(),
            AssociationConfig::quiet(),
            Box::new(NullHandler),
            Box::new(NullHandler),
            Box::new(NullHandler),
        )
        .await
        .unwrap();

    tokio::time::advance(Duration::from_secs(9)).await;

    // while a request to the second outstation is outstanding, the harness outstation sends a
    // response, which refreshes its keep-alive
    let restart = tokio::spawn(async move { other.cold_restart().await });
    harness.expect_write(vec![0xC0, COLD_RESTART]).await;
    harness
        .process_response(empty_response(Sequence::new(0)))
        .await;
    assert_eq!(restart.await.unwrap(), Err(TaskError::ResponseTimeout));

    // so a read issued after the original keep-alive deadline runs at once, instead of waiting
    // behind a link status request
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
    assert_eq!(read.await.unwrap(), Ok(()));
    assert_eq!(now.elapsed(), Duration::ZERO);
}
