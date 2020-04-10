use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::master::handlers::NullResponseHandler;
use dnp3rs::master::runner::TaskRunner;
use dnp3rs::master::task::MasterTask;
use dnp3rs::master::types::ClassScan;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let mut socket = TcpStream::connect(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let (mut reader, mut writer) = dnp3rs::transport::create_transport_layer(true, 1);

    let mut runner = TaskRunner::new(Duration::from_secs(1), NullResponseHandler::create());

    loop {
        let mut task =
            MasterTask::class_scan(1024, ClassScan::integrity(), NullResponseHandler::create());
        runner
            .run(
                ParseLogLevel::ObjectValues,
                &mut socket,
                &mut task,
                &mut writer,
                &mut reader,
            )
            .await
            .unwrap();
        tokio::time::delay_for(Duration::from_secs(2)).await;
    }
}
