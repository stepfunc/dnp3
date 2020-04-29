use dnp3::app::enums::OpType;
use dnp3::app::parse::DecodeLogLevel;
use dnp3::app::timeout::Timeout;
use dnp3::app::variations::Group12Var1;
use dnp3::master::association::{Association, Configuration};
use dnp3::master::handle::{Listener, NullHandler};
use dnp3::master::request::{
    Classes, CommandBuilder, CommandMode, EventClasses, ReadRequest, TimeSyncProcedure,
};
use dnp3::master::tcp::ReconnectStrategy;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

fn get_association() -> Association {
    let mut config = Configuration::default();
    config.auto_time_sync = Some(TimeSyncProcedure::LAN);
    let mut association = Association::new(1024, config, NullHandler::boxed());
    association.add_poll(
        ReadRequest::ClassScan(Classes::events(EventClasses::all())),
        Duration::from_secs(5),
    );
    association
}

/// example of using the master API asynchronously from within the Tokio runtime
#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    // spawn the master onto another task
    let mut master = dnp3::master::tcp::spawn(
        1,
        DecodeLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Timeout::from_secs(1).unwrap(),
        SocketAddr::from_str("127.0.0.1:20000")?,
        Listener::None,
    );

    let mut association = master.add_association(get_association()).await?;

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
