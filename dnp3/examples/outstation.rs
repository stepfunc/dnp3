//! Example master application
use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::app::*;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::outstation::database::*;
use dnp3::outstation::*;

use dnp3::app::attr::{AttrProp, Attribute, StringAttr};
use dnp3::tcp::*;
use std::process::exit;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_util::codec::FramedRead;
use tokio_util::codec::LinesCodec;

#[cfg(feature = "serial")]
use dnp3::serial::*;
#[cfg(feature = "tls")]
use dnp3::tcp::tls::*;
use dnp3::udp::{spawn_outstation_udp, OutstationUdpConfig, UdpSocketMode};

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
        "tcp" => run_tcp_server().await,
        "tcp-client" => run_tcp_client().await,
        "udp" => run_udp().await,
        #[cfg(feature = "serial")]
        "serial" => run_serial().await,
        #[cfg(feature = "tls")]
        "tls-ca" => run_tls_server(get_ca_chain_config()?).await,
        #[cfg(feature = "tls")]
        "tls-self-signed" => run_tls_server(get_self_signed_config()?).await,
        _ => {
            eprintln!(
                "unknown transport '{}', options are (tcp, serial, tls-ca, tls-self-signed)",
                transport
            );
            exit(-1);
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

async fn run_tcp_server() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tcp_server
    let server = Server::new_tcp_server(LinkErrorMode::Close, "127.0.0.1:20000".parse()?);
    // ANCHOR_END: create_tcp_server

    run_server(server).await
}

async fn run_udp() -> Result<(), Box<dyn std::error::Error>> {
    let udp_config = OutstationUdpConfig {
        local_endpoint: "127.0.0.1:20000".parse().unwrap(),
        remote_endpoint: "127.0.0.1:20001".parse().unwrap(),
        socket_mode: UdpSocketMode::OneToOne,
        link_read_mode: LinkReadMode::Datagram,
        retry_delay: Timeout::from_secs(5)?,
    };

    let outstation = spawn_outstation_udp(
        udp_config,
        get_outstation_config(),
        Box::new(ExampleOutstationApplication),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
    );

    run_outstation(outstation).await
}

async fn run_tcp_client() -> Result<(), Box<dyn std::error::Error>> {
    let outstation = spawn_outstation_tcp_client(
        LinkErrorMode::Close,
        EndpointList::single("127.0.0.1:20000".to_string()),
        ConnectStrategy::default(),
        ConnectOptions::default(),
        get_outstation_config(),
        Box::new(ExampleOutstationApplication),
        Box::new(ExampleOutstationInformation),
        Box::new(ExampleControlHandler),
        NullListener::create(),
    );

    run_outstation(outstation).await
}

#[cfg(feature = "serial")]
async fn run_serial() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_serial_server
    let outstation = spawn_outstation_serial_2(
        // change this for a real port
        "/dev/ttySIM1",
        SerialSettings::default(),
        get_outstation_config(),
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

#[cfg(feature = "tls")]
async fn run_tls_server(config: TlsServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: create_tls_server
    let server = Server::new_tls_server(LinkErrorMode::Close, "127.0.0.1:20001".parse()?, config);
    // ANCHOR_END: create_tls_server

    run_server(server).await
}

async fn run_server(mut server: Server) -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR: tcp_server_spawn_outstation
    let outstation = server.add_outstation(
        get_outstation_config(),
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

#[cfg(feature = "tls")]
fn get_ca_chain_config() -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_ca_chain_config
    let config = TlsServerConfig::full_pki(
        Some("test.com".to_string()),
        Path::new("./certs/ca_chain/ca_cert.pem"),
        Path::new("./certs/ca_chain/entity2_cert.pem"),
        Path::new("./certs/ca_chain/entity2_key.pem"),
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_ca_chain_config

    Ok(config)
}

#[cfg(feature = "tls")]
fn get_self_signed_config() -> Result<TlsServerConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_self_signed_config
    let config = TlsServerConfig::self_signed(
        Path::new("./certs/self_signed/entity1_cert.pem"),
        Path::new("./certs/self_signed/entity2_cert.pem"),
        Path::new("./certs/self_signed/entity2_key.pem"),
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_self_signed_config

    Ok(config)
}

fn get_outstation_config() -> OutstationConfig {
    // ANCHOR: outstation_config
    // create an outstation configuration with default values
    let mut config = OutstationConfig::new(
        // outstation address
        EndpointAddress::try_new(1024).unwrap(),
        // master address
        EndpointAddress::try_new(1).unwrap(),
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
