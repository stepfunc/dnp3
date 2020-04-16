use dnp3rs::app::gen::enums::{OpType, TripCloseCode};
use dnp3rs::app::gen::variations::fixed::Group12Var1;
use dnp3rs::app::gen::variations::variation::Variation;
use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::app::types::ControlCode;
use dnp3rs::master::handlers::{NullReadHandler, RequestCompletionHandler};
use dnp3rs::master::runner::{RequestError, RequestRunner};
use dnp3rs::master::session::Session;
use dnp3rs::master::types::*;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;

struct LoggingHandler;
impl CommandTaskHandler for LoggingHandler {
    fn on_response(&mut self, result: Result<(), CommandResponseError>) {
        match result {
            Err(err) => log::warn!("command error: {}", err),
            Ok(()) => log::info!("command request succeeded"),
        }
    }
}
impl RequestCompletionHandler for LoggingHandler {
    fn on_complete(&mut self, result: Result<(), RequestError>) {
        if let Err(err) = result {
            log::warn!("task error: {}", err)
        }
    }
}

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let mut socket = TcpStream::connect(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let (mut reader, mut writer) = dnp3rs::transport::create_transport_layer(true, 1);

    let mut runner = RequestRunner::new(
        ParseLogLevel::ObjectValues,
        Duration::from_secs(1),
        NullReadHandler::create(),
    );

    let session = Session::new(1024);

    loop {
        let task1 = session.read(
            ReadRequest::class_scan(Classes::integrity()),
            NullReadHandler::create(),
        );

        let task2 = session.select_before_operate(
            vec![CommandHeader::U8(PrefixedCommandHeader::G12V1(vec![
                (
                    Group12Var1::from_code(ControlCode::from_op_type(OpType::LatchOn)),
                    7,
                ),
                (
                    Group12Var1::from_code(ControlCode::from_tcc_and_op_type(
                        TripCloseCode::Trip,
                        OpType::PulseOn,
                    )),
                    1,
                ),
            ]))],
            Box::new(LoggingHandler {}),
        );

        let task3 = session.read(
            ReadRequest::Range8(RangeScan::new(Variation::Group1Var2, 1, 5)),
            NullReadHandler::create(),
        );

        for task in [task1, task2, task3].iter_mut() {
            runner
                .run(&mut socket, task, &mut writer, &mut reader)
                .await
                .unwrap();
            tokio::time::delay_for(Duration::from_secs(2)).await;
        }
    }
}
