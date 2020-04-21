use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::master::handlers::NullHandler;
use dnp3rs::master::runner::RequestRunner;
use dnp3rs::master::session::{Session, SessionConfig, SessionMap};
use dnp3rs::master::types::{Classes, EventClasses, ReadRequest};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;

fn get_session_map() -> SessionMap {
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

    let mut socket = TcpStream::connect(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let (mut reader, mut writer) = dnp3rs::transport::create_transport_layer(true, 1);

    let mut runner = RequestRunner::new(
        ParseLogLevel::ObjectValues,
        Duration::from_secs(1),
        get_session_map(),
    );

    runner
        .run_tasks(&mut socket, &mut writer, &mut reader)
        .await
        .unwrap();

    Ok(())
}
