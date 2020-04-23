use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::master::association::{Association, AssociationConfig, SessionMap};
use dnp3rs::master::handlers::NullHandler;
use dnp3rs::master::tcp::{MasterTask, ReconnectStrategy};
use dnp3rs::master::types::{Classes, EventClasses, ReadRequest};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::stream::StreamExt;

use tokio_util::codec::{FramedRead, LinesCodec};

fn get_sessions() -> SessionMap {
    let mut sessions = SessionMap::new();
    let mut session = Association::new(1024, AssociationConfig::default(), NullHandler::boxed());
    session.add_poll(
        ReadRequest::ClassScan(Classes::events(EventClasses::all())),
        Duration::from_secs(5),
    );
    sessions.register(session);
    sessions
}

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    // spawn the master onto another task
    let _handle = MasterTask::spawn(
        1,
        ParseLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Duration::from_secs(1),
        SocketAddr::from_str("127.0.0.1:20000")?,
        get_sessions(),
    );

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            s => println!("unknown command: {}", s),
        }
    }
}
