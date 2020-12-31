use dnp3::app::enums::CommandStatus;
use dnp3::app::flags::Flags;
use dnp3::app::measurement::*;
use dnp3::app::parse::DecodeLogLevel;
use dnp3::entry::outstation::any_address;
use dnp3::entry::outstation::tcp::TCPServer;
use dnp3::entry::EndpointAddress;
use dnp3::outstation::config::OutstationConfig;
use dnp3::outstation::database::config::*;
use dnp3::outstation::database::EventClass;
use dnp3::outstation::database::{Add, DatabaseConfig, Update, UpdateOptions};
use dnp3::outstation::task::OutstationTask;
use dnp3::outstation::traits::{
    DefaultControlHandler, DefaultOutstationApplication, DefaultOutstationInformation,
};
use std::time::Duration;

fn get_database_config() -> DatabaseConfig {
    let mut config = DatabaseConfig::default();
    config.events.max_analog = 10;
    config
}

/// example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let outstation_address = EndpointAddress::from(1024)?;
    let master_address = EndpointAddress::from(1)?;

    let mut config = OutstationConfig::new(outstation_address, master_address);
    config.log_level = DecodeLogLevel::ObjectValues;

    let (task, handle) = OutstationTask::create(
        config,
        get_database_config(),
        DefaultOutstationApplication::create(),
        DefaultOutstationInformation::create(),
        DefaultControlHandler::with_status(CommandStatus::Success),
    );

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

    let mut server = TCPServer::bind("127.0.0.1:20000".parse()?).await?;

    // spawn the outstation and the server
    tokio::spawn(server.add_outstation(task, handle.clone(), any_address(0)));
    let (_server_handle, server) = server.build();
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
