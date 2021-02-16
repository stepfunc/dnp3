use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::decode::*;
use dnp3::link::{EndpointAddress, LinkErrorMode};
use dnp3::outstation::database::*;
use dnp3::outstation::tcp::{AddressFilter, TcpServer};
use dnp3::outstation::OutstationConfig;
use dnp3::outstation::*;

use std::time::Duration;

fn get_outstation_config() -> OutstationConfig {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    let mut config = OutstationConfig::new(
        // outstation address
        EndpointAddress::from(1024).unwrap(),
        // master address
        EndpointAddress::from(1).unwrap(),
    );
    // override the default decoding
    config.decode_level.application = AppDecodeLevel::ObjectValues;
    // ANCHOR_END: outstation_config
    config
}

/// example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let mut server = TcpServer::new(LinkErrorMode::Close, "127.0.0.1:20000".parse()?);

    let handle = server.spawn_outstation(
        get_outstation_config(),
        // event buffer space for 100 analog events
        EventBufferConfig::new(0, 0, 0, 0, 0, 100, 0, 0),
        // customizable trait that controls outstation behavior
        DefaultOutstationApplication::create(),
        // customizable trait to receive events about what the outstation is doing
        DefaultOutstationInformation::create(),
        // customizable trait to process control requests from the master
        DefaultControlHandler::with_status(CommandStatus::NotSupported),
        // filter that controls what IP address(es) may connect to this outstation instance
        AddressFilter::Any,
    )?;

    // setup the outstation's database before we spawn it
    handle.database.transaction(|db| {
        for i in 0..10 {
            db.add(i, Some(EventClass::Class1), AnalogConfig::default());
            db.update(
                i,
                &Analog::new(10.0, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::initialize(),
            );
        }
    });

    // dropping the ServerHandle shuts down the server AND the outstation
    let _server_handle = server.bind_and_spawn().await?;

    let mut value = 0.0;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        handle.database.transaction(|db| {
            db.update(
                7,
                &Analog::new(value, Flags::new(0x01), Time::synchronized(1)),
                UpdateOptions::default(),
            )
        });
        value += 1.0;
    }
}
