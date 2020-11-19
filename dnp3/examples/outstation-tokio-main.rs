use dnp3::app::flags::Flags;
use dnp3::app::measurement::*;
use dnp3::app::parse::DecodeLogLevel;
use dnp3::entry::NormalAddress;
use dnp3::outstation::database::config::*;
use dnp3::outstation::database::EventClass;
use dnp3::outstation::database::{Add, DatabaseConfig, Update, UpdateOptions};
use dnp3::outstation::task::{OutstationConfig, OutstationTask};
use std::net::Ipv4Addr;
use std::time::Duration;

fn get_outstation_config() -> OutstationConfig {
    OutstationConfig::new(
        2048,
        NormalAddress::from(10).unwrap(),
        Some(1),
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

    let (mut task, mut handle) =
        OutstationTask::create(get_outstation_config(), get_database_config());

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
