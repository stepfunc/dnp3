use std::time::Duration;

use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;
use dnp3::serial::*;

fn get_serial_port_path() -> String {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        "/dev/pts/4".to_string()
    } else {
        args[1].clone()
    }
}

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

    let outstation = spawn_outstation_serial(
        &get_serial_port_path(),
        SerialSettings::default(),
        get_outstation_config(),
        // event buffer space for 100 analog events
        EventBufferConfig::new(0, 0, 0, 0, 0, 100, 0, 0),
        // customizable trait that controls outstation behavior
        DefaultOutstationApplication::create(),
        // customizable trait to receive events about what the outstation is doing
        DefaultOutstationInformation::create(),
        // customizable trait to process control requests from the master
        DefaultControlHandler::with_status(CommandStatus::NotSupported),
    )?;

    // setup the outstation's database before we spawn it
    outstation.transaction(|db| {
        for i in 0..10 {
            db.add(i, Some(EventClass::Class1), AnalogConfig::default());
            db.update(
                i,
                &Analog::new(10.0, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::initialize(),
            );
        }
    });

    let mut value = 0.0;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        outstation.transaction(|db| {
            db.update(
                7,
                &Analog::new(value, Flags::new(0x01), Time::synchronized(1)),
                UpdateOptions::default(),
            )
        });
        value += 1.0;
    }
}
