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
use std::io::BufRead;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

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

/// example of using the master API synchronously from outside the Tokio runtime
fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    // create
    let (future, mut master) = dnp3::master::tcp::create(
        1,
        DecodeLogLevel::ObjectValues,
        ReconnectStrategy::default(),
        Timeout::from_secs(1).unwrap(),
        SocketAddr::from_str("127.0.0.1:20000")?,
        Listener::None,
    );

    runtime.spawn(future);

    let mut association = runtime.block_on(master.add_association(get_association()))?;

    loop {
        match std::io::stdin()
            .lock()
            .lines()
            .next()
            .unwrap()
            .unwrap()
            .as_str()
        {
            "x" => return Ok(()),
            "dln" => runtime.block_on(master.set_decode_log_level(DecodeLogLevel::Nothing)),
            "dlv" => runtime.block_on(master.set_decode_log_level(DecodeLogLevel::ObjectValues)),
            "cmd" => {
                if let Err(err) = runtime.block_on(association.operate(
                    CommandMode::SelectBeforeOperate,
                    CommandBuilder::single_u16_header(
                        Group12Var1::from_op_type(OpType::LatchOn),
                        3u16,
                    ),
                )) {
                    log::warn!("error: {}", err);
                }
            }
            "lts" => {
                if let Err(err) =
                    runtime.block_on(association.perform_time_sync(TimeSyncProcedure::LAN))
                {
                    log::warn!("error: {}", err);
                }
            }
            "nts" => {
                if let Err(err) =
                    runtime.block_on(association.perform_time_sync(TimeSyncProcedure::NonLAN))
                {
                    log::warn!("error: {}", err);
                }
            }
            s => println!("unknown command: {}", s),
        }
    }
}
