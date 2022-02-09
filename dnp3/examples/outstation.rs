use std::time::Duration;

use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::app::NullListener;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;
use dnp3::serial::{spawn_outstation_serial, SerialSettings};
use dnp3::tcp::*;
use std::path::Path;
use std::process::exit;

#[cfg(feature = "tls")]
use dnp3::tcp::tls::*;

/// example of using the outstation API asynchronously from within the Tokio runtime
///
/// The application takes a single command line argument specifying the desired transport
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let args: Vec<String> = std::env::args().collect();
    let transport: &str = match args.as_slice() {
        [_, x] => x,
        _ => {
            eprintln!("please specify a transport:");
            eprintln!("usage: outstation <transport> (tcp, serial, tls-ca, tls-self-signed)");
            exit(-1);
        }
    };
    match transport {
        "tcp" => run_tcp().await,
        "serial" => run_serial().await,
        #[cfg(feature = "tls")]
        "tls-ca" => run_tls(get_ca_chain_config()?).await,
        #[cfg(feature = "tls")]
        "tls-self-signed" => run_tls(get_self_signed_config()?).await,
        _ => {
            eprintln!(
                "unknown transport '{}', options are (tcp, serial, tls-ca, tls-self-signed)",
                transport
            );
            exit(-1);
        }
    }
}

async fn run_tcp() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tcp_server
    let server = TcpServer::new(LinkErrorMode::Close, "127.0.0.1:20000".parse()?);
    // ANCHOR_END: create_tcp_server

    run_tcp_server(server).await
}

async fn run_serial() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_serial_server
    let outstation = spawn_outstation_serial(
        // change this for a real port
        "/dev/ttySIM1",
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
    // ANCHOR_END: create_serial_server

    run_outstation(outstation).await
}

#[cfg(feature = "tls")]
async fn run_tls(config: TlsServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tls_server
    let server =
        TcpServer::new_tls_server(LinkErrorMode::Close, "127.0.0.1:20001".parse()?, config);
    // ANCHOR_END: create_tls_server

    run_tcp_server(server).await
}

async fn run_tcp_server(mut server: TcpServer) -> Result<(), Box<dyn std::error::Error>> {
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
            db.add(i, Some(EventClass::Class1), AnalogInputConfig::default());
            db.update(
                i,
                &AnalogInput::new(10.0, Flags::ONLINE, Time::synchronized(0)),
                UpdateOptions::initialize(),
            );
        }
    });
    // ANCHOR_END: database_init

    // ANCHOR: server_bind
    // dropping the ServerHandle shuts down the server and outstation(s)
    let _server_handle = server.bind().await?;
    // ANCHOR_END: server_bind

    run_outstation(outstation).await
}

// run the same logic regardless of the transport type
async fn run_outstation(outstation: OutstationHandle) -> Result<(), Box<dyn std::error::Error>> {
    let mut value = 0.0;

    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        outstation.transaction(|db| {
            db.update(
                7,
                &AnalogInput::new(value, Flags::new(0x01), Time::synchronized(1)),
                UpdateOptions::default(),
            )
        });
        value += 1.0;
    }
}

#[cfg(feature = "tls")]
fn get_ca_chain_config() -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_ca_chain_config
    let config = TlsServerConfig::new(
        "test.com",
        &Path::new("./certs/ca_chain/ca_cert.pem"),
        &Path::new("./certs/ca_chain/entity2_cert.pem"),
        &Path::new("./certs/ca_chain/entity2_key.pem"),
        None, // no password
        MinTlsVersion::V12,
        CertificateMode::AuthorityBased,
    )?;
    // ANCHOR_END: tls_ca_chain_config

    Ok(config)
}

#[cfg(feature = "tls")]
fn get_self_signed_config() -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_self_signed_config
    let config = TlsServerConfig::new(
        "test.com",
        &Path::new("./certs/self_signed/entity1_cert.pem"),
        &Path::new("./certs/self_signed/entity2_cert.pem"),
        &Path::new("./certs/self_signed/entity2_key.pem"),
        None, // no password
        MinTlsVersion::V12,
        CertificateMode::SelfSigned,
    )?;
    // ANCHOR_END: tls_self_signed_config

    Ok(config)
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

// ANCHOR: event_buffer_config
fn get_event_buffer_config() -> EventBufferConfig {
    // initialize the config to zero for every type
    let mut config = EventBufferConfig::no_events();
    // event buffer space for 100 analog events
    config.max_analog = 100;
    config
}
// ANCHOR_END: event_buffer_config