use crate::link::EndpointAddress;
use crate::outstation::tests::harness::*;
use crate::outstation::Feature;

#[tokio::test]
async fn ignores_message_sent_from_master_different_than_configured_by_default() {
    let config = get_default_config();
    let different_address =
        EndpointAddress::try_new(config.master_address.raw_value() + 1).unwrap();
    let mut harness = new_harness_with_master_addr(config, different_address);

    harness.send_and_process(&[0xC0, 0x01]).await;
    harness.expect_no_response();
}

#[tokio::test]
async fn answers_message_sent_from_master_different_than_configured_when_enabled() {
    let mut config = get_default_config();
    config.features.respond_to_any_master = Feature::Enabled;
    let different_address =
        EndpointAddress::try_new(config.master_address.raw_value() + 1).unwrap();
    let mut harness = new_harness_with_master_addr(config, different_address);

    harness
        .test_request_response(&[0xC0, 0x01], &[0xC0, 0x81, 0x80, 0x00])
        .await;
}
