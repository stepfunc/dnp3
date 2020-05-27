use dnp3::prelude::master::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

/// example of using the master API asynchronously from within the Tokio runtime
#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    // spawn the master onto another task
    let mut master = spawn_master_tcp_client(
        1,
        DecodeLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Timeout::from_secs(1)?,
        SocketAddr::from_str("127.0.0.1:20000")?,
        Listener::None,
    );

    // Create the association
    let mut config = Configuration::default();
    config.auto_time_sync = Some(TimeSyncProcedure::LAN);
    let mut association = master
        .add_association(1024, config, NullHandler::boxed())
        .await?;

    // Create event poll
    let mut poll = association
        .add_poll(EventClasses::all().to_request(), Duration::from_secs(5))
        .await?;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "dln" => master.set_decode_log_level(DecodeLogLevel::Nothing).await,
            "dlv" => {
                master
                    .set_decode_log_level(DecodeLogLevel::ObjectValues)
                    .await
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
                    log::warn!("error: {}", err);
                }
            }
            "evt" => poll.demand().await,
            "lts" => {
                if let Err(err) = association.perform_time_sync(TimeSyncProcedure::LAN).await {
                    log::warn!("error: {}", err);
                }
            }
            "nts" => {
                if let Err(err) = association
                    .perform_time_sync(TimeSyncProcedure::NonLAN)
                    .await
                {
                    log::warn!("error: {}", err);
                }
            }
            s => println!("unknown command: {}", s),
        }
    }
}
