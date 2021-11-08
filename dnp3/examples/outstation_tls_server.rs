use std::path::Path;
use std::time::Duration;

use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::app::NullListener;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;
use dnp3::tcp::tls::*;
use dnp3::tcp::*;

fn get_outstation_config() -> OutstationConfig {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    let mut config = OutstationConfig::new(
        // outstation address
        EndpointAddress::from(10).unwrap(),
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

    #[allow(unused)]
    // ANCHOR: tls_self_signed_config
    let self_signed_tls_config = TlsServerConfig::new(
        "test.com",
        &Path::new("./certs/self_signed/entity1_cert.pem"),
        &Path::new("./certs/self_signed/entity2_cert.pem"),
        &Path::new("./certs/self_signed/entity2_key.pem"),
        None, // no password
        MinTlsVersion::Tls1_2,
        CertificateMode::SelfSignedCertificate,
    )?;
    // ANCHOR_END: tls_self_signed_config

    #[allow(unused)]
    // ANCHOR: tls_ca_chain_config
    let ca_chain_tls_config = TlsServerConfig::new(
        "test.com",
        &Path::new("./certs/ca_chain/ca_cert.pem"),
        &Path::new("./certs/ca_chain/entity2_cert.pem"),
        &Path::new("./certs/ca_chain/entity2_key.pem"),
        None, // no password
        MinTlsVersion::Tls1_2,
        CertificateMode::TrustChain,
    )?;
    // ANCHOR_END: tls_ca_chain_config

    let tls_config = ca_chain_tls_config;

    // ANCHOR: create_tls_server
    let mut server =
        TcpServer::new_tls_server(LinkErrorMode::Close, "127.0.0.1:20001".parse()?, tls_config);
    // ANCHOR_END: create_tls_server

    // ANCHOR: tcp_server_spawn_outstation
    let outstation = server.add_outstation(
        get_outstation_config(),
        get_event_buffer_config(),
        DefaultOutstationApplication::create(),
        DefaultOutstationInformation::create(),
        DefaultControlHandler::with_status(CommandStatus::NotSupported),
        NullListener::create(),
        AddressFilter::Any,
    )?;
    // ANCHOR_END: tcp_server_spawn_outstation

    // setup the outstation's database before we spawn it
    // ANCHOR: database_init
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
    // ANCHOR_END: database_init

    // ANCHOR: server_bind
    // dropping the ServerHandle shuts down the server and outstation(s)
    let _server_handle = server.bind().await?;
    // ANCHOR_END: server_bind

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
