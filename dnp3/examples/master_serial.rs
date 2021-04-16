use std::time::Duration;

use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

use dnp3::app::control::*;
use dnp3::app::*;
use dnp3::decode::*;
use dnp3::link::EndpointAddress;
use dnp3::master::*;
use dnp3::serial::*;

fn get_serial_port_path() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        "/dev/pts/4".to_string()
    } else {
        args[1].clone()
    }
}

fn get_master_channel_config() -> Result<MasterChannelConfig, Box<dyn std::error::Error>> {
    let mut config = MasterChannelConfig::new(EndpointAddress::from(1)?);
    config.decode_level = AppDecodeLevel::ObjectValues.into();
    config.response_timeout = Timeout::from_secs(1)?;
    Ok(config)
}

/// example of using the master from within the Tokio runtime
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // spawn the master onto another task
    let mut master = spawn_master_serial(
        get_master_channel_config()?,
        &get_serial_port_path(),
        SerialSettings::default(),
        Duration::from_secs(1),
        Listener::None,
    );

    // Create the association
    let mut config = AssociationConfig::default();
    config.auto_time_sync = Some(TimeSyncProcedure::Lan);
    config.keep_alive_timeout = Some(Duration::from_secs(60));
    let mut association = master
        .add_association(
            EndpointAddress::from(1024)?,
            config,
            NullHandler::boxed(),
            NullHandler::boxed(),
        )
        .await?;

    // Create event poll
    let mut poll = association
        .add_poll(
            EventClasses::all().to_classes().to_request(),
            Duration::from_secs(5),
        )
        .await?;

    master.enable().await?;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "enable" => {
                master.enable().await?;
            }
            "disable" => {
                master.disable().await?;
            }
            "dln" => {
                master.set_decode_level(DecodeLevel::nothing()).await?;
            }
            "dlv" => {
                master
                    .set_decode_level(AppDecodeLevel::ObjectValues.into())
                    .await?;
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
                        CommandBuilder::single_header_u16(
                            Group12Var1::from_op_type(OpType::LatchOn),
                            3u16,
                        ),
                    )
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "evt" => poll.demand().await?,
            "lts" => {
                if let Err(err) = association.synchronize_time(TimeSyncProcedure::Lan).await {
                    tracing::warn!("error: {}", err);
                }
            }
            "nts" => {
                if let Err(err) = association
                    .synchronize_time(TimeSyncProcedure::NonLan)
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
