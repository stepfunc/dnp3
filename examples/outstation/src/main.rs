//! Example outstation application
use clap::{Parser, Subcommand};
use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::app::*;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;

use dnp3::app::attr::{AttrProp, Attribute, StringAttr};
use dnp3::tcp::*;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;

use dnp3::serial::*;
use dnp3::tcp::tls::*;
use dnp3::udp::{spawn_outstation_udp, OutstationUdpConfig, UdpSocketMode};
use dnp3_cli_utils::serial::{DataBitsArg, FlowControlArg, ParityArg, StopBitsArg};
use dnp3_cli_utils::LogLevel;

/// DNP3 Outstation example application

#[derive(Debug, Parser)]
#[command(name = "outstation")]
#[command(about = "DNP3 Outstation example application", long_about = None)]
struct CliArgs {
    /// Log level to use
    #[arg(short, long, value_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,

    /// Outstation address (DNP3 address of the outstation)
    #[arg(short, long, default_value = "1024")]
    outstation_address: EndpointAddress,

    /// Master address (DNP3 address of the master)
    #[arg(short, long, default_value = "1")]
    master_address: EndpointAddress,

    #[command(subcommand)]
    transport: TransportCommand,
}

#[derive(Debug, Subcommand)]
enum TransportCommand {
    /// Use TCP server transport
    TcpServer {
        /// IP address and port to bind to
        #[arg(short, long, default_value = "127.0.0.1:20000")]
        endpoint: SocketAddr,
    },
    /// Use TCP client transport
    TcpClient {
        /// IP address and port to connect to
        #[arg(short, long, default_value = "127.0.0.1:20000")]
        endpoint: SocketAddr,
    },
    /// Use UDP transport
    Udp {
        /// Local IP address and port to bind to
        #[arg(short, long, default_value = "127.0.0.1:20000")]
        local_endpoint: SocketAddr,

        /// Remote IP address and port to send to
        #[arg(short, long, default_value = "127.0.0.1:20001")]
        remote_endpoint: SocketAddr,
    },

    /// Use serial transport
    Serial {
        /// Serial port name
        #[arg(short, long, default_value = "/dev/ttyS0")]
        port: String,

        /// Baud rate
        #[arg(short, long, default_value = "9600")]
        baud_rate: u32,

        /// Data bits
        #[arg(long, value_enum, default_value_t = DataBitsArg::Eight)]
        data_bits: DataBitsArg,

        /// Stop bits
        #[arg(long, value_enum, default_value_t = StopBitsArg::One)]
        stop_bits: StopBitsArg,

        /// Parity
        #[arg(long, value_enum, default_value_t = ParityArg::None)]
        parity: ParityArg,

        /// Flow control
        #[arg(long, value_enum, default_value_t = FlowControlArg::None)]
        flow_control: FlowControlArg,
    },

    /// Use TLS with CA chain transport
    TlsCa {
        /// IP address and port to bind to
        #[arg(short, long, default_value = "127.0.0.1:20001")]
        endpoint: SocketAddr,

        /// Domain name to verify
        #[arg(long, default_value = "test.com")]
        domain: String,

        /// Path to CA certificate file
        #[arg(long, default_value = "./certs/ca_chain/ca_cert.pem")]
        ca_cert: PathBuf,

        /// Path to entity certificate file
        #[arg(long, default_value = "./certs/ca_chain/entity2_cert.pem")]
        entity_cert: PathBuf,

        /// Path to entity private key file
        #[arg(long, default_value = "./certs/ca_chain/entity2_key.pem")]
        entity_key: PathBuf,
    },

    /// Use TLS with self-signed certificates
    TlsSelfSigned {
        /// IP address and port to bind to
        #[arg(short, long, default_value = "127.0.0.1:20001")]
        endpoint: SocketAddr,

        /// Path to peer certificate file
        #[arg(long, default_value = "./certs/self_signed/entity1_cert.pem")]
        peer_cert: PathBuf,

        /// Path to entity certificate file
        #[arg(long, default_value = "./certs/self_signed/entity2_cert.pem")]
        entity_cert: PathBuf,

        /// Path to entity private key file
        #[arg(long, default_value = "./certs/self_signed/entity2_key.pem")]
        entity_key: PathBuf,
    },
}

/// Example of using the outstation API asynchronously from within the Tokio runtime
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = CliArgs::parse();

    // Initialize logging
    let log_level: tracing::Level = args.log_level.into();

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();

    // Process the transport command
    match &args.transport {
        TransportCommand::TcpServer { endpoint } => {
            tracing::info!("Starting TCP server on {}", endpoint);
            run_tcp_server(*endpoint, &args).await
        }
        TransportCommand::TcpClient { endpoint } => {
            tracing::info!("Starting TCP client to {}", endpoint);
            run_tcp_client(*endpoint, &args).await
        }
        TransportCommand::Udp {
            local_endpoint,
            remote_endpoint,
        } => {
            tracing::info!(
                "Starting UDP transport from {} to {}",
                local_endpoint,
                remote_endpoint
            );
            run_udp(*local_endpoint, *remote_endpoint, &args).await
        }

        TransportCommand::Serial {
            port,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            flow_control,
        } => {
            tracing::info!("Starting serial transport on {}", port);
            run_serial(
                port,
                *baud_rate,
                *data_bits,
                *stop_bits,
                *parity,
                *flow_control,
                &args,
            )
            .await
        }

        TransportCommand::TlsCa {
            endpoint,
            domain,
            ca_cert,
            entity_cert,
            entity_key,
        } => {
            tracing::info!("Starting TLS server with CA chain on {}", endpoint);
            let config = get_ca_chain_config(domain, ca_cert, entity_cert, entity_key)?;
            run_tls_server(*endpoint, config, &args).await
        }

        TransportCommand::TlsSelfSigned {
            endpoint,
            peer_cert,
            entity_cert,
            entity_key,
        } => {
            tracing::info!(
                "Starting TLS server with self-signed certificates on {}",
                endpoint
            );
            let config = get_self_signed_config(peer_cert, entity_cert, entity_key)?;
            run_tls_server(*endpoint, config, &args).await
        }
    }
}

struct ExampleOutstationApplication;
impl OutstationApplication for ExampleOutstationApplication {
    fn support_write_analog_dead_bands(&mut self) -> bool {
        true
    }

    fn write_analog_dead_band(&mut self, index: u16, dead_band: f64) {
        tracing::info!("change analog dead-band {index} to {dead_band}");
    }

    fn write_device_attr(&mut self, attr: Attribute) -> MaybeAsync<bool> {
        tracing::info!("write device attribute: {:?}", attr);
        // Allow writing any attribute that has been defined as writable
        MaybeAsync::ready(true)
    }
}

struct ExampleOutstationInformation;
impl OutstationInformation for ExampleOutstationInformation {}

// ANCHOR: control_handler
struct ExampleControlHandler;
impl ControlHandler for ExampleControlHandler {}

impl ControlSupport<Group12Var1> for ExampleControlHandler {
    fn select(
        &mut self,
        control: Group12Var1,
        index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        if index < 10
            && (control.code.op_type == OpType::LatchOn || control.code.op_type == OpType::LatchOff)
        {
            CommandStatus::Success
        } else {
            CommandStatus::NotSupported
        }
    }

    fn operate(
        &mut self,
        control: Group12Var1,
        index: u16,
        _op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        if index < 10
            && (control.code.op_type == OpType::LatchOn || control.code.op_type == OpType::LatchOff)
        {
            let status = control.code.op_type == OpType::LatchOn;
            database.transaction(|db| {
                db.update(
                    index,
                    &BinaryOutputStatus::new(status, Flags::ONLINE, get_current_time()),
                    UpdateOptions::detect_event(),
                );
            });
            CommandStatus::Success
        } else {
            CommandStatus::NotSupported
        }
    }
}

impl ExampleControlHandler {
    fn select_analog_output(&self, index: u16) -> CommandStatus {
        if index < 10 {
            CommandStatus::Success
        } else {
            CommandStatus::NotSupported
        }
    }

    fn operate_analog_output(
        &self,
        value: f64,
        index: u16,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        if index < 10 {
            database.transaction(|db| {
                db.update(
                    index,
                    &AnalogOutputStatus::new(value, Flags::ONLINE, get_current_time()),
                    UpdateOptions::detect_event(),
                );
            });
            CommandStatus::Success
        } else {
            CommandStatus::NotSupported
        }
    }
}

impl ControlSupport<Group41Var1> for ExampleControlHandler {
    fn select(
        &mut self,
        _control: Group41Var1,
        index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.select_analog_output(index)
    }

    fn operate(
        &mut self,
        control: Group41Var1,
        index: u16,
        _op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.operate_analog_output(control.value as f64, index, database)
    }
}

impl ControlSupport<Group41Var2> for ExampleControlHandler {
    fn select(
        &mut self,
        _control: Group41Var2,
        index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.select_analog_output(index)
    }

    fn operate(
        &mut self,
        control: Group41Var2,
        index: u16,
        _op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.operate_analog_output(control.value as f64, index, database)
    }
}

impl ControlSupport<Group41Var3> for ExampleControlHandler {
    fn select(
        &mut self,
        _control: Group41Var3,
        index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.select_analog_output(index)
    }

    fn operate(
        &mut self,
        control: Group41Var3,
        index: u16,
        _op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.operate_analog_output(control.value as f64, index, database)
    }
}

impl ControlSupport<Group41Var4> for ExampleControlHandler {
    fn select(
        &mut self,
        _control: Group41Var4,
        index: u16,
        _database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.select_analog_output(index)
    }

    fn operate(
        &mut self,
        control: Group41Var4,
        index: u16,
        _op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        self.operate_analog_output(control.value, index, database)
    }
}
// ANCHOR_END: control_handler

async fn run_tcp_server(
    endpoint: SocketAddr,
    cli: &CliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tcp_server
    let server = Server::new_tcp_server(LinkErrorMode::Close, endpoint);
    // ANCHOR_END: create_tcp_server

    run_server(server, cli).await
}

async fn run_udp(
    local_endpoint: SocketAddr,
    remote_endpoint: SocketAddr,
    cli: &CliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let udp_config = OutstationUdpConfig {
        local_endpoint,
        remote_endpoint,
        socket_mode: UdpSocketMode::OneToOne,
        link_read_mode: LinkReadMode::Datagram,
        retry_delay: Timeout::from_secs(5)?,
    };

    let outstation = spawn_outstation_udp(
        udp_config,
        get_outstation_config_from_cli(cli),
        Box::new(ExampleOutstationApplication),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
    );

    run_outstation(outstation).await
}

async fn run_tcp_client(
    endpoint: SocketAddr,
    cli: &CliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let outstation = spawn_outstation_tcp_client(
        LinkErrorMode::Close,
        EndpointList::single(endpoint.to_string()),
        ConnectStrategy::default(),
        ConnectOptions::default(),
        get_outstation_config_from_cli(cli),
        Box::new(ExampleOutstationApplication),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
        NullListener::create(),
    );

    run_outstation(outstation).await
}

async fn run_serial(
    port: &str,
    baud_rate: u32,
    data_bits: DataBitsArg,
    stop_bits: StopBitsArg,
    parity: ParityArg,
    flow_control: FlowControlArg,
    cli: &CliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_serial_server
    // Setup serial settings with values from Clap
    let settings = SerialSettings {
        baud_rate,
        data_bits: data_bits.into(),
        stop_bits: stop_bits.into(),
        parity: parity.into(),
        flow_control: flow_control.into(),
    };

    let outstation = spawn_outstation_serial_2(
        port,
        settings,
        get_outstation_config_from_cli(cli),
        RetryStrategy::new(Duration::from_secs(1), Duration::from_secs(60)),
        // customizable trait that controls outstation behavior
        Box::new(ExampleOutstationApplication),
        // customizable trait to receive events about what the outstation is doing
        Box::new(ExampleOutstationInformation),
        // customizable trait to process control requests from the master
        Box::new(ExampleControlHandler),
        NullListener::create(),
    );
    // ANCHOR_END: create_serial_server

    run_outstation(outstation).await
}

async fn run_tls_server(
    endpoint: SocketAddr,
    config: TlsServerConfig,
    cli: &CliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tls_server
    let server = Server::new_tls_server(LinkErrorMode::Close, endpoint, config);
    // ANCHOR_END: create_tls_server

    run_server(server, cli).await
}

async fn run_server(mut server: Server, cli: &CliArgs) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: tcp_server_spawn_outstation
    let outstation = server.add_outstation(
        get_outstation_config_from_cli(cli),
        Box::new(ExampleOutstationApplication),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
        NullListener::create(),
        AddressFilter::Any,
    )?;
    // ANCHOR_END: tcp_server_spawn_outstation

    // set up the outstation's database before we spawn it
    // ANCHOR: database_init
    outstation.transaction(|db| {
        // initialize 10 points of each type
        for i in 0..10 {
            db.add(
                i,
                Some(EventClass::Class1),
                // you can explicitly specify the configuration for each point ...
                BinaryInputConfig {
                    s_var: StaticBinaryInputVariation::Group1Var1,
                    e_var: EventBinaryInputVariation::Group2Var2,
                },
            );
            db.add(
                i,
                Some(EventClass::Class1),
                // ... or just use the defaults
                DoubleBitBinaryInputConfig::default(),
            );
            db.add(
                i,
                Some(EventClass::Class1),
                BinaryOutputStatusConfig::default(),
            );
            db.add(i, Some(EventClass::Class1), CounterConfig::default());
            db.add(i, Some(EventClass::Class1), FrozenCounterConfig::default());
            db.add(
                i,
                Some(EventClass::Class1),
                AnalogInputConfig {
                    s_var: StaticAnalogInputVariation::Group30Var1,
                    e_var: EventAnalogInputVariation::Group32Var1,
                    deadband: 0.0,
                },
            );
            db.add(
                i,
                Some(EventClass::Class1),
                AnalogOutputStatusConfig::default(),
            );
            db.add(i, Some(EventClass::Class1), OctetStringConfig);
        }

        // define device attributes made available to the master
        let _ = db.define_attr(
            AttrProp::default(),
            StringAttr::DeviceManufacturersName.with_value("Step Function I/O"),
        );
        let _ = db.define_attr(
            AttrProp::writable(),
            StringAttr::UserAssignedLocation.with_value("Bend, OR"),
        );
    });
    // ANCHOR_END: database_init

    // ANCHOR: server_bind
    // dropping the ServerHandle shuts down the server and outstation(s)
    let _server_handle = server.bind().await?;
    // ANCHOR_END: server_bind

    run_outstation(outstation).await
}

// run the same logic regardless of the transport type
async fn run_outstation(
    mut outstation: OutstationHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut binary_input_value = false;
    let mut double_bit_binary_input_value = DoubleBit::DeterminedOff;
    let mut binary_output_status_value = false;
    let mut counter_value = 0;
    let mut frozen_counter_value = 0;
    let mut analog_input_value = 0.0;
    let mut analog_output_status_value = 0.0;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    tracing::info!("Outstation started. Available commands:");
    tracing::info!("  x - exit");
    tracing::info!("  enable - enable the outstation");
    tracing::info!("  disable - disable the outstation");
    tracing::info!("  bi - toggle binary input");
    tracing::info!("  dbbi - toggle double bit binary input");
    tracing::info!("  bos - toggle binary output status");
    tracing::info!("  co - increment counter");
    tracing::info!("  fco - increment frozen counter");
    tracing::info!("  ai - increment analog input");
    tracing::info!("  aif - set analog input flag to COMM_LOST");
    tracing::info!("  aos - increment analog output status");
    tracing::info!("  os - set octet string to 'Hello'");

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "enable" => {
                outstation.enable().await?;
            }
            "disable" => {
                outstation.disable().await?;
            }
            "bi" => {
                binary_input_value = !binary_input_value;
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &BinaryInput::new(binary_input_value, Flags::ONLINE, get_current_time()),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "dbbi" => {
                double_bit_binary_input_value =
                    if double_bit_binary_input_value == DoubleBit::DeterminedOff {
                        DoubleBit::DeterminedOn
                    } else {
                        DoubleBit::DeterminedOff
                    };
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &DoubleBitBinaryInput::new(
                            double_bit_binary_input_value,
                            Flags::ONLINE,
                            get_current_time(),
                        ),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "bos" => {
                binary_output_status_value = !binary_output_status_value;
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &BinaryOutputStatus::new(
                            binary_output_status_value,
                            Flags::ONLINE,
                            get_current_time(),
                        ),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "co" => {
                counter_value += 1;
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &Counter::new(counter_value, Flags::ONLINE, get_current_time()),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "fco" => {
                frozen_counter_value += 1;
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &FrozenCounter::new(
                            frozen_counter_value,
                            Flags::ONLINE,
                            get_current_time(),
                        ),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "ai" => {
                analog_input_value += 1.0;
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &AnalogInput::new(analog_input_value, Flags::ONLINE, get_current_time()),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "aif" => outstation.transaction(|db| {
                db.update_flags(
                    7,
                    UpdateFlagsType::AnalogInput,
                    Flags::COMM_LOST,
                    Some(get_current_time()),
                    UpdateOptions::detect_event(),
                );
            }),
            "aos" => {
                analog_output_status_value += 1.0;
                outstation.transaction(|db| {
                    db.update(
                        7,
                        &AnalogOutputStatus::new(
                            analog_output_status_value,
                            Flags::ONLINE,
                            get_current_time(),
                        ),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "os" => outstation.transaction(|db| {
                db.update(
                    7,
                    &OctetString::new("Hello".as_bytes()).unwrap(),
                    UpdateOptions::detect_event(),
                );
            }),
            s => println!("unknown command: {}", s),
        }
    }
}

fn get_current_time() -> Time {
    let epoch_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    Time::Synchronized(Timestamp::new(epoch_time.as_millis() as u64))
}

fn get_ca_chain_config(
    domain: &str,
    ca_cert: &Path,
    entity_cert: &Path,
    entity_key: &Path,
) -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_ca_chain_config
    let config = TlsServerConfig::full_pki(
        Some(domain.to_string()),
        ca_cert,
        entity_cert,
        entity_key,
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_ca_chain_config

    Ok(config)
}

fn get_self_signed_config(
    peer_cert: &Path,
    entity_cert: &Path,
    entity_key: &Path,
) -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_self_signed_config
    let config = TlsServerConfig::self_signed(
        peer_cert,
        entity_cert,
        entity_key,
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_self_signed_config

    Ok(config)
}

fn get_outstation_config_from_cli(cli: &CliArgs) -> OutstationConfig {
    // ANCHOR: outstation_config
    // create an outstation configuration with values from CLI
    let mut config = OutstationConfig::new(
        // outstation address
        cli.outstation_address,
        // master address
        cli.master_address,
        get_event_buffer_config(),
    );
    config.class_zero.octet_string = true;

    // override the default decoding
    config.decode_level.application = AppDecodeLevel::ObjectValues;
    // ANCHOR_END: outstation_config
    config
}

// ANCHOR: event_buffer_config
fn get_event_buffer_config() -> EventBufferConfig {
    EventBufferConfig::new(
        10, // binary
        10, // double-bit binary
        10, // binary output status
        5,  // counter
        5,  // frozen counter
        5,  // analog
        5,  // analog output status
        3,  // octet string
    )
}
// ANCHOR_END: event_buffer_config
