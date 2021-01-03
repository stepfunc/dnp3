use dnp3::app::enums::CommandStatus;
use dnp3::app::flags::Flags;
use dnp3::app::measurement::*;
use dnp3::app::parse::DecodeLogLevel;
use dnp3::entry::outstation::tcp::TCPServer;
use dnp3::entry::outstation::AddressFilter;
use dnp3::entry::EndpointAddress;
use dnp3::outstation::config::OutstationConfig;
use dnp3::outstation::database::config::*;
use dnp3::outstation::database::EventClass;
use dnp3::outstation::database::{Add, DatabaseConfig, Update, UpdateOptions};
use dnp3::outstation::traits::{
    DefaultControlHandler, DefaultOutstationApplication, DefaultOutstationInformation,
};
use std::time::Duration;

fn get_database_config() -> DatabaseConfig {
    let mut config = DatabaseConfig::default();
    config.events.max_analog = 10;
    config
}

fn get_outstation_config() -> OutstationConfig {
    let outstation_address = EndpointAddress::from(1024).unwrap();
    let master_address = EndpointAddress::from(1).unwrap();
    let mut config = OutstationConfig::new(outstation_address, master_address);
    config.log_level = DecodeLogLevel::ObjectValues;
    config
}

/// example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let mut server = TCPServer::new("127.0.0.1:20000".parse()?);

    let (handle, outstation) = server.add_outstation(
        get_outstation_config(),
        get_database_config(),
        DefaultOutstationApplication::create(),
        DefaultOutstationInformation::create(),
        DefaultControlHandler::with_status(CommandStatus::Success),
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
    let (_server_handle, server) = server.bind().await?;

    // spawn the outstation and the server
    tokio::spawn(outstation);
    tokio::spawn(server);

    let mut value = 0.0;

    loop {
        tokio::time::delay_for(Duration::from_secs(5)).await;
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
