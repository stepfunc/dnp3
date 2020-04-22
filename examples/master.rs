use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::master::handlers::NullHandler;
use dnp3rs::master::session::{Session, SessionConfig, SessionMap};
use dnp3rs::master::tcp::{MasterTask, ReconnectStrategy};
use dnp3rs::master::types::{Classes, EventClasses, ReadRequest};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

fn get_sessions() -> SessionMap {
    let mut sessions = SessionMap::new();
    let mut session = Session::new(1024, SessionConfig::default(), NullHandler::boxed());
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

    let address = SocketAddr::from_str("127.0.0.1:20000")?;

    let (mut task, _handle) = MasterTask::new(
        1,
        ParseLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Duration::from_secs(1),
        address,
        get_sessions(),
    );

    task.run().await.ok();

    Ok(())
}
