use dnp3::app::flags::Flags;
use dnp3::app::measurement::{Binary, Time};
use dnp3::outstation::database::EventBufferConfig;
use dnp3::outstation::database::{Update, UpdateOptions};
use dnp3::outstation::task::OutstationTask;
use dnp3::outstation::types::EventClass;
use dnp3::outstation::variations::{EventBinaryVariation, StaticBinaryVariation};
use std::net::Ipv4Addr;
use std::time::Duration;

/// example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(threaded_scheduler)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    let (mut task, mut handle) = OutstationTask::create(10, EventBufferConfig::uniform(10));

    handle.update(|db| {
        for i in 0..10 {
            db.add_binary(
                i,
                EventClass::Class1,
                StaticBinaryVariation::Group1Var1,
                EventBinaryVariation::Group2Var1,
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

    let mut value = true;

    loop {
        tokio::time::delay_for(Duration::from_secs(5)).await;
        handle.update(|db| {
            db.update(
                &Binary::new(value, Flags::new(0x01), Time::synchronized(1)),
                7,
                UpdateOptions::default(),
            )
        });
        value = !value;
    }
}
