use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::master::association::{Association, AssociationConfig, AssociationMap};
use dnp3rs::master::handlers::NullHandler;
use dnp3rs::master::runner::CommandMode;
use dnp3rs::master::tcp::{MasterTask, ReconnectStrategy};
use dnp3rs::master::types::{Classes, EventClasses, ReadRequest};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

fn get_association() -> Association {
    let mut association =
        Association::new(1024, AssociationConfig::default(), NullHandler::boxed());
    association.add_poll(
        ReadRequest::ClassScan(Classes::events(EventClasses::all())),
        Duration::from_secs(5),
    );
    association
}

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    // spawn the master onto another task
    let mut handle = MasterTask::spawn(
        1,
        ParseLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Duration::from_secs(1),
        SocketAddr::from_str("127.0.0.1:20000")?,
        AssociationMap::single(get_association()),
    );

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "c" => {
                match handle
                    .operate(1024, CommandMode::DirectOperate, vec![])
                    .await
                {
                    Ok(()) => log::info!("success"),
                    Err(err) => log::warn!("error: {}", err),
                }
            }
            s => println!("unknown command: {}", s),
        }
    }
}
