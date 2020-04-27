use dnp3rs::app::gen::enums::OpType;
use dnp3rs::app::gen::variations::fixed::Group12Var1;
use dnp3rs::app::parse::parser::DecodeLogLevel;
use dnp3rs::master::association::{Association, AssociationConfig};
use dnp3rs::master::null::NullHandler;
use dnp3rs::master::tcp::{MasterTask, ReconnectStrategy};
use dnp3rs::master::types::{Classes, CommandBuilder, CommandMode, EventClasses, ReadRequest};
use dnp3rs::util::timeout::Timeout;
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
    let mut master = MasterTask::spawn(
        1,
        DecodeLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Timeout::from_secs(1).unwrap(),
        SocketAddr::from_str("127.0.0.1:20000")?,
    );

    let mut association = master.add_association(get_association()).await?;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "c" => {
                match association
                    .operate(
                        CommandMode::SelectBeforeOperate,
                        CommandBuilder::single(Group12Var1::from_op_type(OpType::LatchOn), 3u16),
                    )
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
