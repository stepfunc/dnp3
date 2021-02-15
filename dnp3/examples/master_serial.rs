use dnp3::app::timeout::Timeout;
use dnp3::app::variations::{Group12Var1, Variation};
use dnp3::app::OpType;
use dnp3::app::ReconnectStrategy;
use dnp3::decode::*;
use dnp3::link::EndpointAddress;
use dnp3::master::association::AssociationConfig;
use dnp3::master::handle::{Listener, MasterConfig, NullHandler};
use dnp3::master::request::{
    CommandBuilder, CommandMode, EventClasses, ReadHeader, ReadRequest, TimeSyncProcedure,
};
use dnp3::master::serial::SerialSettings;

use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

/// example of using the master API asynchronously from within the Tokio runtime
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // spawn the master onto another task
    let mut master = dnp3::master::serial::spawn_master_serial_client(
        MasterConfig::new(
            EndpointAddress::from(1)?,
            AppDecodeLevel::ObjectValues.into(),
            ReconnectStrategy::default(),
            Timeout::from_secs(1)?,
        ),
        "/dev/pts/4",
        SerialSettings::default(),
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
