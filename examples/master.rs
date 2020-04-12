use dnp3rs::app::gen::enums::{CommandStatus, OpType, TripCloseCode};
use dnp3rs::app::gen::variations::fixed::Group12Var1;
use dnp3rs::app::parse::parser::ParseLogLevel;
use dnp3rs::app::types::ControlCode;
use dnp3rs::master::handlers::NullResponseHandler;
use dnp3rs::master::runner::TaskRunner;
use dnp3rs::master::task::MasterTask;
use dnp3rs::master::types::{CommandHeader, PrefixedCommandHeader};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::net::TcpStream;

#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let mut socket = TcpStream::connect(SocketAddr::from_str("127.0.0.1:20000")?).await?;

    let (mut reader, mut writer) = dnp3rs::transport::create_transport_layer(true, 1);

    let mut runner = TaskRunner::new(
        ParseLogLevel::ObjectValues,
        Duration::from_secs(1),
        NullResponseHandler::create(),
    );

    let crob = Group12Var1 {
        code: ControlCode {
            tcc: TripCloseCode::Nul,
            clear: false,
            queue: false,
            op_type: OpType::LatchOn,
        },
        count: 1,
        on_time: 0,
        off_time: 0,
        status: CommandStatus::Success,
    };

    loop {
        /*
        let mut task =
            MasterTask::class_scan(1024, ClassScan::integrity(), NullResponseHandler::create());
        */

        let mut task = MasterTask::command(
            1024,
            vec![CommandHeader::U8(PrefixedCommandHeader::G12V1(vec![
                (crob, 7),
                (crob, 1),
            ]))],
        );

        runner
            .run(&mut socket, &mut task, &mut writer, &mut reader)
            .await
            .unwrap();
        tokio::time::delay_for(Duration::from_secs(2)).await;
    }
}
