use std::time::Duration;

use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;
use dnp3::tcp::*;

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

// ANCHOR: event_buffer_config
fn get_event_buffer_config() -> EventBufferConfig {
    // initialize the config to zero for every type
    let mut config = EventBufferConfig::no_events();
    // event buffer space for 100 analog events
    config.max_analog = 100;
    config
}
// ANCHOR_END: event_buffer_config

/// example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    // ANCHOR: create_tcp_server
    let mut server = TcpServer::new(LinkErrorMode::Close, "127.0.0.1:20000".parse()?);
    // ANCHOR_END: create_tcp_server

    // ANCHOR: tcp_server_spawn_outstation
    let outstation = server.spawn_outstation(
        get_outstation_config(),
        get_event_buffer_config(),
        DefaultOutstationApplication::create(),
        DefaultOutstationInformation::create(),
        DefaultControlHandler::with_status(CommandStatus::NotSupported),
        // filter that controls what IP address(es) may connect to this outstation instance
        AddressFilter::Any,
    )?;
    // ANCHOR_END: tcp_server_spawn_outstation

    // setup the outstation's database before we spawn it
    // ANCHOR: database_init
    outstation.database.transaction(|db| {
        for i in 0..10 {
            db.add(i, Some(EventClass::Class1), AnalogConfig::default());
            db.update(
                i,
                &Analog::new(10.0, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::initialize(),
            );
        }
    });
    // ANCHOR_END: database_init

    // ANCHOR: server_bind
    // dropping the ServerHandle shuts down the server and outstation(s)
    let _server_handle = server.bind_and_spawn().await?;
    // ANCHOR_END: server_bind

    let mut value = 0.0;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        outstation.database.transaction(|db| {
            db.update(
                7,
                &Analog::new(value, Flags::new(0x01), Time::synchronized(1)),
                UpdateOptions::default(),
            )
        });
        value += 1.0;
    }
}
