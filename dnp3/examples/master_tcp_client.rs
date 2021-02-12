use dnp3::config::EndpointAddress;
use dnp3::config::{AppDecodeLevel, LinkErrorMode};
use dnp3::prelude::master::*;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

/*
  Example of using the master API from within the Tokio runtime.
  The program initializes a master and then enters a loop reading console input
  allowing the user to perform common tasks interactively.
*/
// ANCHOR: runtime_init
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR_END: runtime_init

    // ANCHOR: logging
    // initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    // ANCHOR_END: logging

    // spawn the master onto another task
    let mut master = spawn_master_tcp_client(
        LinkErrorMode::Close,
        MasterConfiguration::new(
            EndpointAddress::from(1)?,
            AppDecodeLevel::ObjectValues.into(),
            ReconnectStrategy::default(),
            Timeout::from_secs(1)?,
        ),
        EndpointList::new("127.0.0.1:20000".to_owned(), &[]),
        Listener::None,
    );

    // Create the association
    let mut config = AssociationConfig::default();
    config.auto_time_sync = Some(TimeSyncProcedure::Lan);
    config.keep_alive_timeout = Some(Duration::from_secs(60));
    let mut association = master
        .add_association(EndpointAddress::from(1024)?, config, NullHandler::boxed())
        .await?;

    // Create event poll
    let mut poll = association
        .add_poll(
            EventClasses::all().to_classes().to_request(),
            Duration::from_secs(5),
        )
        .await?;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "dln" => {
                master.set_decode_level(DecodeLevel::nothing()).await.ok();
            }
            "dlv" => {
                master
                    .set_decode_level(AppDecodeLevel::ObjectValues.into())
                    .await
                    .ok();
            }
            "rao" => {
                if let Err(err) = association
                    .read(ReadRequest::all_objects(Variation::Group40Var0))
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "rmo" => {
                if let Err(err) = association
                    .read(ReadRequest::multiple_headers(&[
                        ReadHeader::all_objects(Variation::Group10Var0),
                        ReadHeader::all_objects(Variation::Group40Var0),
                    ]))
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "cmd" => {
                if let Err(err) = association
                    .operate(
                        CommandMode::SelectBeforeOperate,
                        CommandBuilder::single_u16_header(
                            Group12Var1::from_op_type(OpType::LatchOn),
                            3u16,
                        ),
                    )
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "evt" => poll.demand().await,
            "lts" => {
                if let Err(err) = association.perform_time_sync(TimeSyncProcedure::Lan).await {
                    tracing::warn!("error: {}", err);
                }
            }
            "nts" => {
                if let Err(err) = association
                    .perform_time_sync(TimeSyncProcedure::NonLan)
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "crt" => {
                if let Err(err) = association.cold_restart().await {
                    tracing::warn!("error: {}", err);
                }
            }
            "wrt" => {
                if let Err(err) = association.warm_restart().await {
                    tracing::warn!("error: {}", err);
                }
            }
            "lsr" => {
                tracing::info!("{:?}", association.check_link_status().await);
            }
            s => println!("unknown command: {}", s),
        }
    }
}
