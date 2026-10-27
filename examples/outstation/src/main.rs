// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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

/// Number of points of each type in the example's database
const POINT_COUNT: u16 = 10;

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

async fn run_tcp_server(
    endpoint: SocketAddr,
    cli: &CliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tcp_server
    let server = Server::new_tcp_server(LinkErrorMode::Close, endpoint);
    // ANCHOR_END: create_tcp_server

    run_server(server, cli).await
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
        Box::new(ExampleOutstationApplication::new()),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
        LogListener::create("client"),
    );

    initialize_database(&outstation);
    run_outstation(outstation).await
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
        Box::new(ExampleOutstationApplication::new()),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
    );

    initialize_database(&outstation);
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
        Box::new(ExampleOutstationApplication::new()),
        // customizable trait to receive events about what the outstation is doing
        Box::new(ExampleOutstationInformation),
        // customizable trait to process control requests from the master
        Box::new(ExampleControlHandler),
        LogListener::create("port"),
    );
    // ANCHOR_END: create_serial_server

    initialize_database(&outstation);
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
    // ANCHOR: tcp_server_add_outstation
    let outstation = server.add_outstation(
        get_outstation_config_from_cli(cli),
        Box::new(ExampleOutstationApplication::new()),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
        LogListener::create("connection"),
        AddressFilter::Any,
    )?;
    // ANCHOR_END: tcp_server_add_outstation

    // set up the outstation's database before the server starts accepting connections
    initialize_database(&outstation);

    // ANCHOR: tcp_server_bind
    // dropping the ServerHandle shuts down the server and outstation(s)
    let _server_handle = server.bind().await?;
    // ANCHOR_END: tcp_server_bind

    run_outstation(outstation).await
}

/// Add the example's points and attributes to the database
fn initialize_database(outstation: &OutstationHandle) {
    // ANCHOR: database_init
    outstation.transaction(|db| {
        // initialize POINT_COUNT points of each type
        for i in 0..POINT_COUNT {
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
}

// run the same logic regardless of the transport type
async fn run_outstation(
    mut outstation: OutstationHandle,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut binary_input_value = false;
    let mut double_bit_binary_input_value = DoubleBit::DeterminedOff;
    let mut binary_output_status_value = false;
    let mut analog_input_value = 0.0;
    let mut analog_output_status_value = 0.0;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    print_help();

    // exit on "x" or at the end of input
    while let Some(line) = reader.next().await {
        match line?.as_str() {
            "x" => return Ok(()),
            "help" => print_help(),
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
                outstation.transaction(|db| {
                    // read the counter, which a freeze may have cleared
                    let value = Get::<Counter>::get(db, 7).map_or(0, |c| c.value);
                    db.update(
                        7,
                        &Counter::new(value.wrapping_add(1), Flags::ONLINE, get_current_time()),
                        UpdateOptions::detect_event(),
                    );
                })
            }
            "fco" => outstation.transaction(|db| {
                // read the frozen counter, which a freeze may have changed
                let value = Get::<FrozenCounter>::get(db, 7).map_or(0, |c| c.value);
                db.update(
                    7,
                    &FrozenCounter::new(value.wrapping_add(1), Flags::ONLINE, get_current_time()),
                    UpdateOptions::detect_event(),
                );
            }),
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
            s => println!("unknown command: {s}"),
        }
    }
    Ok(())
}

fn print_help() {
    println!("Commands (each updates the point at index 7):");
    println!("  help - print this list");
    println!("  x - exit");
    println!("  enable - enable the outstation");
    println!("  disable - disable the outstation");
    println!("  bi - toggle binary input");
    println!("  dbbi - toggle double bit binary input");
    println!("  bos - toggle binary output status");
    println!("  co - increment counter");
    println!("  fco - increment frozen counter");
    println!("  ai - increment analog input");
    println!("  aif - set analog input flag to COMM_LOST");
    println!("  aos - increment analog output status");
    println!("  os - set octet string to 'Hello'");
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

struct ExampleOutstationApplication {
    need_time: bool,
}

impl ExampleOutstationApplication {
    fn new() -> Self {
        // ask the master for the time as soon as it connects
        Self { need_time: true }
    }
}

impl OutstationApplication for ExampleOutstationApplication {
    // ANCHOR: time_sync
    fn get_application_iin(&self) -> ApplicationIin {
        ApplicationIin {
            need_time: self.need_time,
            ..Default::default()
        }
    }

    fn write_absolute_time(&mut self, time: Timestamp) -> Result<(), RequestError> {
        // a real device sets its clock here, and sets need_time again when its clock drifts
        tracing::info!("write absolute time: {time}");
        self.need_time = false;
        Ok(())
    }
    // ANCHOR_END: time_sync

    // ANCHOR: freeze
    fn freeze_counter(
        &mut self,
        indices: FreezeIndices,
        freeze_type: FreezeType,
        database: &mut DatabaseHandle,
    ) -> Result<(), RequestError> {
        let clear = match freeze_type {
            FreezeType::ImmediateFreeze => false,
            FreezeType::FreezeAndClear => true,
            // freezing at a time and interval isn't supported
            _ => return Err(RequestError::NotSupported),
        };
        let (start, stop) = match indices {
            FreezeIndices::All => (0, POINT_COUNT - 1),
            FreezeIndices::Range(start, stop) => (start, stop.min(POINT_COUNT - 1)),
        };
        tracing::info!("freeze counters {start} to {stop} (clear: {clear})");
        database.transaction(|db| {
            for index in start..=stop {
                if let Some(counter) = Get::<Counter>::get(db, index) {
                    let time = get_current_time();
                    let frozen = FrozenCounter::new(counter.value, counter.flags, time);
                    db.update(index, &frozen, UpdateOptions::detect_event());
                    if clear {
                        let cleared = Counter::new(0, counter.flags, time);
                        db.update(index, &cleared, UpdateOptions::detect_event());
                    }
                }
            }
        });
        Ok(())
    }
    // ANCHOR_END: freeze

    // ANCHOR: save_changes
    fn support_write_analog_dead_bands(&mut self) -> bool {
        true
    }

    fn begin_write_analog_dead_bands(&mut self) {
        tracing::info!("begin writing analog dead-bands");
    }

    fn write_analog_dead_band(&mut self, index: u16, dead_band: f64) {
        tracing::info!("change analog dead-band {index} to {dead_band}");
    }

    fn end_write_analog_dead_bands(&mut self) -> MaybeAsync<()> {
        tracing::info!("end writing analog dead-bands: save them here");
        MaybeAsync::ready(())
    }

    fn begin_assign_class(&mut self) {
        tracing::info!("begin assigning classes");
    }

    fn assign_class(
        &mut self,
        point_type: PointType,
        indices: PointIndices,
        class: Option<EventClass>,
    ) {
        tracing::info!("assign class {class:?} to {point_type:?} {indices:?}");
    }

    fn end_assign_class(&mut self) -> MaybeAsync<()> {
        tracing::info!("end assigning classes: save them here");
        MaybeAsync::ready(())
    }
    // ANCHOR_END: save_changes

    fn write_device_attr(&mut self, attr: Attribute) -> MaybeAsync<bool> {
        tracing::info!("write device attribute: {:?}", attr);
        // Allow writing any attribute that has been defined as writable
        MaybeAsync::ready(true)
    }
}

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
        if index < POINT_COUNT
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
        op_type: OperateType,
        database: &mut DatabaseHandle,
    ) -> CommandStatus {
        tracing::info!(
            "operate {:?} on binary output {index} via {op_type:?}",
            control.code.op_type
        );
        if index < POINT_COUNT
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
        if index < POINT_COUNT {
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
        if index < POINT_COUNT {
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

struct ExampleOutstationInformation;
impl OutstationInformation for ExampleOutstationInformation {}

/// Logs each state change of a connection, client or serial port
struct LogListener {
    name: &'static str,
}

impl LogListener {
    fn create<T: std::fmt::Debug>(name: &'static str) -> Box<dyn Listener<T>> {
        Box::new(Self { name })
    }
}

impl<T: std::fmt::Debug> Listener<T> for LogListener {
    fn update(&mut self, value: T) -> MaybeAsync<()> {
        tracing::info!("{} state: {value:?}", self.name);
        MaybeAsync::ready(())
    }
}

fn get_current_time() -> Time {
    let epoch_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    Time::Synchronized(Timestamp::new(epoch_time.as_millis() as u64))
}
