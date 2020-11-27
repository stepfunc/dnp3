use dnp3::app::flags::Flags;
use dnp3::app::measurement::*;
use dnp3::app::parse::DecodeLogLevel;
use dnp3::entry::EndpointAddress;
use dnp3::outstation::database::config::*;
use dnp3::outstation::database::EventClass;
use dnp3::outstation::database::{Add, DatabaseConfig, Update, UpdateOptions};
use dnp3::outstation::task::{OutstationConfig, OutstationTask};
use dnp3::outstation::traits::NullHandler;
use dnp3::outstation::SelfAddressSupport;
use std::net::Ipv4Addr;
use std::time::Duration;

fn get_outstation_config(outstation: EndpointAddress, master: EndpointAddress) -> OutstationConfig {
    OutstationConfig::new(
        2048,
        outstation,
        master,
        SelfAddressSupport::Disabled,
        DecodeLogLevel::ObjectValues,
        Duration::from_secs(2),
    )
}

fn get_database_config() -> DatabaseConfig {
    let mut config = DatabaseConfig::default();
    config.events.max_analog = 10;
    config
}

/// example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let outstation_address = EndpointAddress::from(1024)?;
    let master_address = EndpointAddress::from(1)?;

    let (mut task, mut handle) = OutstationTask::create(
        get_outstation_config(outstation_address, master_address),
        get_database_config(),
        NullHandler::create(),
    );

    handle.transaction(|db| {
        for i in 0..1000 {
            db.add(i, Some(EventClass::Class1), AnalogConfig::default());
            db.update(
                i,
                &Analog::new(10.0, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::initialize(),
            );
        }
    });

    let listen_task = async move {
        let mut listener = tokio::net::TcpListener::bind((Ipv4Addr::new(127, 0, 0, 1), 20000))
            .await
            .unwrap();
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();

            let _ = task.run(&mut socket).await;
        }
    };

    tokio::spawn(listen_task);

    let mut value = 0.0;

    loop {
        tokio::time::delay_for(Duration::from_secs(5)).await;
        handle.transaction(|db| {
            db.update(
                7,
                &Analog::new(value, Flags::new(0x01), Time::synchronized(1)),
                UpdateOptions::default(),
            )
        });
        value += 1.0;
    }
}
