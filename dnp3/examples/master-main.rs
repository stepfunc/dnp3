use dnp3::entry::EndpointAddress;
use dnp3::prelude::master::*;
use std::io::BufRead;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;

/// example of using the master API synchronously from outside the Tokio runtime
fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let mut runtime = tokio::runtime::Runtime::new().unwrap();

    // create
    let (future, mut master) = create_master_tcp_client(
        MasterConfiguration::new(
            EndpointAddress::from(1)?,
            DecodeLogLevel::ObjectValues,
            RetryStrategy::default(),
            Timeout::from_secs(1)?,
        ),
        SocketAddr::from_str("127.0.0.1:20000")?,
        Listener::None,
    );

    runtime.spawn(future);

    // Create the association
    let mut config = Configuration::default();
    config.auto_time_sync = Some(TimeSyncProcedure::LAN);
    config.keep_alive_timeout = Some(Duration::from_secs(60));
    let mut association = runtime.block_on(master.add_association(
        EndpointAddress::from(1024)?,
        config,
        NullHandler::boxed(),
    ))?;

    // Add an event poll
    let mut poll = runtime.block_on(association.add_poll(
        EventClasses::all().to_classes().to_request(),
        Duration::from_secs(5),
    ))?;

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
            "dln" => {
                runtime
                    .block_on(master.set_decode_log_level(DecodeLogLevel::Nothing))
                    .ok();
            }
            "dlv" => {
                runtime
                    .block_on(master.set_decode_log_level(DecodeLogLevel::ObjectValues))
                    .ok();
            }
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
            "evt" => {
                runtime.block_on(poll.demand());
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
            "crt" => {
                if let Err(err) = runtime.block_on(association.cold_restart()) {
                    log::warn!("error: {}", err);
                }
            }
            "wrt" => {
                if let Err(err) = runtime.block_on(association.warm_restart()) {
                    log::warn!("error: {}", err);
                }
            }
            "lsr" => {
                log::info!("{:?}", runtime.block_on(association.check_link_status()));
            }
            s => println!("unknown command: {}", s),
        }
    }
}
