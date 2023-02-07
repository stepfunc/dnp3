use crate::link::EndpointAddress;
use crate::outstation::tests::harness::*;

#[tokio::test]
async fn ignores_message_sent_from_master_different_than_configured() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let config = get_default_config();
    let different_address =
        EndpointAddress::try_new(config.master_address.raw_value() + 1).unwrap();
    let mut harness = new_harness_with_master_addr(config, different_address);

    harness.send_and_process(&[0xC0, 0x01]).await;
    harness.expect_no_response();
}
