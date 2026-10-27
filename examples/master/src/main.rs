// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

//! Example master application
use std::time::Duration;

use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

use dnp3::app::attr::*;
use dnp3::app::control::*;
use dnp3::app::measurement::*;
use dnp3::app::*;
use dnp3::decode::*;
use dnp3::link::*;
use dnp3::master::*;
use dnp3::serial::*;
use dnp3::tcp::tls::*;
use dnp3::tcp::*;

use clap::{Parser, Subcommand};
use dnp3::outstation::FreezeInterval;
use dnp3::udp::spawn_master_udp;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use dnp3_cli_utils::serial::{DataBitsArg, FlowControlArg, ParityArg, StopBitsArg};
use dnp3_cli_utils::LogLevel;

/*
  Example program using the master API from within the Tokio runtime.

  The program initializes a master channel based on the command line argument and then enters a loop
  reading console input allowing the user to perform common tasks interactively.
*/
// ANCHOR: runtime_init
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR_END: runtime_init

    // Parse command line arguments
    let args = CliArgs::parse();

    // ANCHOR: logging
    // Initialize logging
    let log_level: tracing::Level = args.log_level.into();

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .init();
    // ANCHOR_END: logging

    if args.parse_zero_length_strings {
        tracing::info!("enabled zero-length octet string parsing");
        options::parse_zero_length_strings(true);
    }

    // spawn the master channel based on the command line argument
    let (mut channel, mut association) = create_channel_and_association(&args).await?;

    // create an event poll
    // ANCHOR: add_poll
    let poll = association
        .add_poll(
            ReadRequest::ClassScan(Classes::class123()),
            Duration::from_secs(5),
        )
        .await?;
    // ANCHOR_END: add_poll

    // enable communications
    channel.enable().await?;

    let mut handler = CliHandler {
        poll,
        channel,
        association,
    };

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    print_help();

    // exit on "x" or at the end of input
    while let Some(line) = reader.next().await {
        match line?.as_str() {
            "x" => return Ok(()),
            "help" => print_help(),
            cmd => match handler.run_one_command(cmd).await {
                Ok(outcome) => println!("{cmd}: {outcome}"),
                Err(err) => println!("{cmd}: {err}"),
            },
        }
    }
    Ok(())
}

#[derive(Debug, Parser)]
#[command(name = "master")]
#[command(about = "DNP3 Master example application", long_about = None)]
struct CliArgs {
    /// Log level to use
    #[arg(short, long, value_enum, default_value_t = LogLevel::Info)]
    log_level: LogLevel,

    /// Master address (DNP3 address of the master)
    #[arg(short, long, default_value = "1")]
    master_address: EndpointAddress,

    /// If true, enable the parsing of zero-length octet strings
    #[arg(short = 'z', long)]
    parse_zero_length_strings: bool,

    #[command(subcommand)]
    transport: TransportCommand,
}

#[derive(Debug, Subcommand)]
enum TransportCommand {
    /// Use TCP client transport
    TcpClient {
        /// IP address and port to connect to
        #[arg(short, long, default_value = "127.0.0.1:20000")]
        endpoint: SocketAddr,

        /// Outstation address (DNP3 address of the outstation)
        #[arg(short, long, default_value = "1024")]
        outstation_address: EndpointAddress,
    },
    /// Use UDP transport
    Udp {
        /// Local IP address and port to bind to
        #[arg(short, long, default_value = "127.0.0.1:20001")]
        local_endpoint: SocketAddr,

        /// Remote IP address and port to send to
        #[arg(short, long, default_value = "127.0.0.1:20000")]
        remote_endpoint: SocketAddr,

        /// Outstation address (DNP3 address of the outstation)
        #[arg(short, long, default_value = "1024")]
        outstation_address: EndpointAddress,
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

        /// Outstation address (DNP3 address of the outstation)
        #[arg(short, long, default_value = "1024")]
        outstation_address: EndpointAddress,
    },

    /// Use TLS with CA chain transport
    TlsCa {
        /// IP address and port to connect to
        #[arg(short, long, default_value = "127.0.0.1:20001")]
        endpoint: SocketAddr,

        /// Domain name to verify
        #[arg(long, default_value = "test.com")]
        domain: String,

        /// Path to CA certificate file
        #[arg(long, default_value = "./certs/ca_chain/ca_cert.pem")]
        ca_cert: PathBuf,

        /// Path to entity certificate file
        #[arg(long, default_value = "./certs/ca_chain/entity1_cert.pem")]
        entity_cert: PathBuf,

        /// Path to entity private key file
        #[arg(long, default_value = "./certs/ca_chain/entity1_key.pem")]
        entity_key: PathBuf,

        /// Outstation address (DNP3 address of the outstation)
        #[arg(short, long, default_value = "1024")]
        outstation_address: EndpointAddress,
    },

    /// Use TLS with self-signed certificates
    TlsSelfSigned {
        /// IP address and port to connect to
        #[arg(short, long, default_value = "127.0.0.1:20001")]
        endpoint: SocketAddr,

        /// Path to peer certificate file
        #[arg(long, default_value = "./certs/self_signed/entity2_cert.pem")]
        peer_cert: PathBuf,

        /// Path to entity certificate file
        #[arg(long, default_value = "./certs/self_signed/entity1_cert.pem")]
        entity_cert: PathBuf,

        /// Path to entity private key file
        #[arg(long, default_value = "./certs/self_signed/entity1_key.pem")]
        entity_key: PathBuf,

        /// Outstation address (DNP3 address of the outstation)
        #[arg(short, long, default_value = "1024")]
        outstation_address: EndpointAddress,
    },
}

// create the specified channel based on the command line argument
async fn create_channel_and_association(
    cli: &CliArgs,
) -> Result<(MasterChannel, AssociationHandle), Box<dyn std::error::Error>> {
    match &cli.transport {
        TransportCommand::TcpClient {
            endpoint,
            outstation_address,
        } => {
            let mut channel = create_tcp_channel(cli.master_address, *endpoint)?;
            let assoc = add_association(&mut channel, *outstation_address).await?;
            Ok((channel, assoc))
        }
        TransportCommand::Udp {
            local_endpoint,
            remote_endpoint,
            outstation_address,
        } => {
            let mut channel = create_udp_channel(cli.master_address, *local_endpoint)?;
            let assoc =
                add_udp_association(&mut channel, *remote_endpoint, *outstation_address).await?;
            Ok((channel, assoc))
        }
        TransportCommand::Serial {
            port,
            baud_rate,
            data_bits,
            stop_bits,
            parity,
            flow_control,
            outstation_address,
        } => {
            let mut channel = create_serial_channel(
                cli.master_address,
                port,
                *baud_rate,
                *data_bits,
                *stop_bits,
                *parity,
                *flow_control,
            )?;
            let assoc = add_association(&mut channel, *outstation_address).await?;
            Ok((channel, assoc))
        }
        TransportCommand::TlsCa {
            endpoint,
            domain,
            ca_cert,
            entity_cert,
            entity_key,
            outstation_address,
        } => {
            let mut channel = create_tls_channel(
                cli.master_address,
                *endpoint,
                get_tls_authority_config(domain, ca_cert, entity_cert, entity_key)?,
            )?;
            let assoc = add_association(&mut channel, *outstation_address).await?;
            Ok((channel, assoc))
        }
        TransportCommand::TlsSelfSigned {
            endpoint,
            peer_cert,
            entity_cert,
            entity_key,
            outstation_address,
        } => {
            let mut channel = create_tls_channel(
                cli.master_address,
                *endpoint,
                get_tls_self_signed_config(peer_cert, entity_cert, entity_key)?,
            )?;
            let assoc = add_association(&mut channel, *outstation_address).await?;
            Ok((channel, assoc))
        }
    }
}

fn create_tcp_channel(
    master_address: EndpointAddress,
    endpoint: SocketAddr,
) -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_tcp_channel
    let channel = spawn_master_tcp_client(
        LinkErrorMode::Close,
        get_master_channel_config(master_address)?,
        EndpointList::new(endpoint.to_string(), &[]),
        ConnectStrategy::default(),
        LogListener::create("client"),
    );
    // ANCHOR_END: create_tcp_channel
    Ok(channel)
}

fn create_udp_channel(
    master_address: EndpointAddress,
    local_endpoint: SocketAddr,
) -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_udp_channel
    let channel = spawn_master_udp(
        local_endpoint,
        LinkReadMode::Datagram,
        Timeout::from_secs(5)?,
        get_master_channel_config(master_address)?,
    );
    // ANCHOR_END: create_udp_channel
    Ok(channel)
}

fn create_serial_channel(
    master_address: EndpointAddress,
    port: &str,
    baud_rate: u32,
    data_bits: DataBitsArg,
    stop_bits: StopBitsArg,
    parity: ParityArg,
    flow_control: FlowControlArg,
) -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_serial_channel
    let settings = SerialSettings {
        baud_rate,
        data_bits: data_bits.into(),
        stop_bits: stop_bits.into(),
        parity: parity.into(),
        flow_control: flow_control.into(),
    };

    let channel = spawn_master_serial(
        get_master_channel_config(master_address)?,
        port,
        settings,
        Duration::from_secs(1),
        LogListener::create("port"),
    );
    // ANCHOR_END: create_serial_channel
    Ok(channel)
}

fn create_tls_channel(
    master_address: EndpointAddress,
    endpoint: SocketAddr,
    config: TlsClientConfig,
) -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_tls_channel
    let channel = spawn_master_tls_client(
        LinkErrorMode::Close,
        get_master_channel_config(master_address)?,
        EndpointList::new(endpoint.to_string(), &[]),
        ConnectStrategy::default(),
        LogListener::create("client"),
        config,
    );
    // ANCHOR_END: create_tls_channel
    Ok(channel)
}

async fn add_association(
    channel: &mut MasterChannel,
    outstation_address: EndpointAddress,
) -> Result<AssociationHandle, Box<dyn std::error::Error>> {
    // ANCHOR: association_create
    let association = channel
        .add_association(
            outstation_address,
            get_association_config(),
            Box::new(ExampleReadHandler),
            Box::new(ExampleAssociationHandler),
            Box::new(ExampleAssociationInformation),
        )
        .await?;
    // ANCHOR_END: association_create
    Ok(association)
}

async fn add_udp_association(
    channel: &mut MasterChannel,
    remote_endpoint: SocketAddr,
    outstation_address: EndpointAddress,
) -> Result<AssociationHandle, Box<dyn std::error::Error>> {
    // ANCHOR: association_create_udp
    let association = channel
        .add_udp_association(
            outstation_address,
            remote_endpoint,
            get_association_config(),
            Box::new(ExampleReadHandler),
            Box::new(ExampleAssociationHandler),
            Box::new(ExampleAssociationInformation),
        )
        .await?;
    // ANCHOR_END: association_create_udp
    Ok(association)
}

struct CliHandler {
    poll: PollHandle,
    channel: MasterChannel,
    association: AssociationHandle,
}

impl CliHandler {
    /// Run a command and describe its outcome
    async fn run_one_command(
        &mut self,
        cmd: &str,
    ) -> Result<&'static str, Box<dyn std::error::Error>> {
        match cmd {
            "enable" => {
                self.channel.enable().await?;
            }
            "disable" => {
                self.channel.disable().await?;
            }
            "dln" => {
                self.channel
                    .set_decode_level(DecodeLevel::nothing())
                    .await?;
            }
            "dlv" => {
                self.channel
                    .set_decode_level(AppDecodeLevel::ObjectValues.into())
                    .await?;
            }
            "rao" => {
                self.association
                    .read(ReadRequest::all_objects(Variation::Group40Var0))
                    .await?;
            }
            "rmo" => {
                self.association
                    .read(ReadRequest::multiple_headers(&[
                        ReadHeader::all_objects(Variation::Group10Var0),
                        ReadHeader::all_objects(Variation::Group40Var0),
                    ]))
                    .await?;
            }
            "rco" => {
                self.association
                    .read(ReadRequest::multiple_headers(&[
                        ReadHeader::all_objects(Variation::Group20Var0),
                        ReadHeader::all_objects(Variation::Group21Var0),
                    ]))
                    .await?;
            }
            "cmd" => {
                // ANCHOR: assoc_control
                self.association
                    .operate(
                        CommandMode::SelectBeforeOperate,
                        CommandBuilder::single_header_u16(
                            Group12Var1::from_op_type(OpType::LatchOn),
                            3u16,
                        ),
                    )
                    .await?;
                // ANCHOR_END: assoc_control
            }
            "cna" => {
                // ANCHOR: assoc_control_no_ack
                self.association
                    .operate_no_ack(CommandBuilder::single_header_u16(
                        Group12Var1::from_op_type(OpType::LatchOn),
                        3u16,
                    ))
                    .await?;
                // ANCHOR_END: assoc_control_no_ack
                // success only means the request was sent: the outstation doesn't answer
            }
            // the poll runs later, and its data goes to the read handler
            "evt" => self.poll.demand().await?,
            "lts" => {
                self.association
                    .synchronize_time(TimeSyncProcedure::Lan)
                    .await?;
            }
            "nts" => {
                self.association
                    .synchronize_time(TimeSyncProcedure::NonLan)
                    .await?;
            }
            "wad" => {
                // ANCHOR: write_dead_bands
                self.association
                    .write_dead_bands(vec![
                        DeadBandHeader::group34_var1_u8(vec![(3, 5)]),
                        DeadBandHeader::group34_var3_u16(vec![(4, 2.5)]),
                    ])
                    .await?
                // ANCHOR_END: write_dead_bands
            }
            "fat" => {
                // ANCHOR: freeze_at_time
                let headers = Headers::new()
                    // freeze all the counters once per day relative to the beginning of the current hour
                    .add_freeze_interval(FreezeInterval::PeriodicallyFreezeRelative(86_400_000))
                    // apply this schedule to all counters
                    .add_all_objects(Variation::Group20Var0);

                self.association
                    .send_and_expect_empty_response(FunctionCode::FreezeAtTime, headers)
                    .await?;
                // ANCHOR_END: freeze_at_time
            }
            "ifz" => {
                // copy every counter to its frozen counter
                let headers = Headers::new().add_all_objects(Variation::Group20Var0);
                self.association
                    .send_and_expect_empty_response(FunctionCode::ImmediateFreeze, headers)
                    .await?;
            }
            "fcl" => {
                // copy every counter to its frozen counter, then clear the counter
                let headers = Headers::new().add_all_objects(Variation::Group20Var0);
                self.association
                    .send_and_expect_empty_response(FunctionCode::FreezeClear, headers)
                    .await?;
            }
            "rda" => {
                // ANCHOR: read_attributes
                self.association
                    .read(ReadRequest::device_attribute(
                        AllAttributes,
                        AttrSet::Default,
                    ))
                    .await?;
                // ANCHOR_END: read_attributes
            }
            "wda" => {
                // ANCHOR: write_attribute
                let headers = Headers::default()
                    .add_attribute(StringAttr::UserAssignedLocation.with_value("Mt. Olympus"));

                self.association
                    .send_and_expect_empty_response(FunctionCode::Write, headers)
                    .await?;
                // ANCHOR_END: write_attribute
            }
            "ral" => {
                self.association
                    .read(ReadRequest::device_attribute(
                        VariationListAttr::ListOfVariations,
                        AttrSet::Default,
                    ))
                    .await?;
            }
            "crt" => {
                let delay = self.association.cold_restart().await?;
                tracing::info!("restart delay: {:?}", delay);
            }
            "wrt" => {
                let delay = self.association.warm_restart().await?;
                tracing::info!("restart delay: {:?}", delay);
            }
            "rd" => {
                // ANCHOR: read_directory
                let items = self
                    .association
                    .read_directory(".", DirReadConfig::default(), None)
                    .await?;

                for info in items {
                    print_file_info(info);
                }
                // ANCHOR_END: read_directory
            }

            "gfi" => {
                // ANCHOR: get_file_info
                let info = self.association.get_file_info(".").await?;
                print_file_info(info);
                // ANCHOR_END: get_file_info
            }
            "rf" => {
                // ANCHOR: read_file
                self.association
                    .read_file(
                        ".", // this reads the root "directory" file but doesn't parse it
                        FileReadConfig::default(),
                        Box::new(FileLogger),
                        None,
                    )
                    .await?;
                // ANCHOR_END: read_file
            }
            "wf" => {
                // ANCHOR: write_file
                let line = "hello world\n".as_bytes();

                let file = self
                    .association
                    .open_file(
                        "hello_world.txt",
                        AuthKey::none(),
                        Permissions::default(),
                        0xFFFFFFFF, // indicate that we don't know the size
                        FileMode::Write,
                        512,
                    )
                    .await?;

                // write the 'hello world' line to the file twice
                let mut block = BlockNumber::default();
                self.association
                    .write_file_block(file.file_handle, block, line.to_vec())
                    .await?;
                block.increment()?;
                block.set_last();
                self.association
                    .write_file_block(file.file_handle, block, line.to_vec())
                    .await?;
                self.association.close_file(file.file_handle).await?;
                // ANCHOR_END: write_file
            }
            "lsr" => self.association.check_link_status().await?,
            _ => return Err("unknown command, type help for a list".into()),
        }
        Ok(match cmd {
            "rf" => "started (the file reader reports the outcome)",
            "cna" => "sent",
            "evt" => "poll demanded",
            _ => "success",
        })
    }
}

fn print_help() {
    println!("Commands:");
    println!("  help - print this list");
    println!("  x - exit");
    println!("  enable / disable - enable or disable the channel");
    println!("  dln / dlv - decode nothing / decode object values");
    println!("  rao - read analog output statuses");
    println!("  rmo - read binary and analog output statuses");
    println!("  rco - read counters and frozen counters");
    println!("  cmd - select and operate binary output 3 (latch on)");
    println!("  cna - operate binary output 3 without an answer");
    println!("  evt - demand the event poll");
    println!("  lts / nts - synchronize the time (LAN / non-LAN)");
    println!("  wad - write analog input deadbands");
    println!("  ifz - freeze all counters");
    println!("  fcl - freeze and clear all counters");
    println!("  fat - freeze all counters once a day");
    println!("  rda - read all device attributes");
    println!("  wda - write the user-assigned location attribute");
    println!("  ral - read the list of attribute variations");
    println!("  crt / wrt - cold / warm restart");
    println!("  rd - read the root directory");
    println!("  gfi - get information about the root directory");
    println!("  rf - read the root directory as a file");
    println!("  wf - write hello_world.txt");
    println!("  lsr - check the link status");
}

// ANCHOR: master_channel_config
fn get_master_channel_config(
    master_address: EndpointAddress,
) -> Result<MasterChannelConfig, Box<dyn std::error::Error>> {
    let mut config = MasterChannelConfig::new(master_address);
    config.decode_level = AppDecodeLevel::ObjectValues.into();
    Ok(config)
}
// ANCHOR_END: master_channel_config

// ANCHOR: association_config
fn get_association_config() -> AssociationConfig {
    let mut config = AssociationConfig::new(
        // disable unsolicited first (Class 1/2/3)
        EventClasses::all(),
        // after the integrity poll, enable unsolicited (Class 1/2/3)
        EventClasses::all(),
        // perform startup integrity poll with Class 1/2/3/0
        Classes::all(),
        // don't automatically scan Class 1/2/3 when the corresponding IIN bit is asserted
        EventClasses::none(),
    );
    config.auto_time_sync = Some(TimeSyncProcedure::Lan);
    config.keep_alive_timeout = Some(Duration::from_secs(60));
    config
}
// ANCHOR_END: association_config

fn get_tls_self_signed_config(
    peer_cert: &Path,
    entity_cert: &Path,
    entity_key: &Path,
) -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_self_signed_config
    let config = TlsClientConfig::self_signed(
        peer_cert,
        entity_cert,
        entity_key,
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_self_signed_config
    Ok(config)
}

fn get_tls_authority_config(
    domain: &str,
    ca_cert: &Path,
    entity_cert: &Path,
    entity_key: &Path,
) -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_ca_chain_config
    let config = TlsClientConfig::full_pki(
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

// ANCHOR: read_handler
struct ExampleReadHandler;

impl ReadHandler for ExampleReadHandler {
    fn begin_fragment(&mut self, read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        println!(
            "begin fragment: {read_type:?} (broadcast: {})",
            header.iin.iin1.get_broadcast()
        );
        MaybeAsync::ready(())
    }

    fn end_fragment(&mut self, _read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        println!("end fragment");
        MaybeAsync::ready(())
    }

    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        print_header("Binary inputs", info);
        for (x, idx) in iter {
            print_point("BI", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
        print_header("Double-bit binary inputs", info);
        for (x, idx) in iter {
            print_point("DBBI", idx, format!("{:?}", x.value), x.flags, x.time);
        }
    }

    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        print_header("Binary output statuses", info);
        for (x, idx) in iter {
            print_point("BOS", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
        print_header("Counters", info);
        for (x, idx) in iter {
            print_point("C", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        print_header("Frozen counters", info);
        for (x, idx) in iter {
            print_point("FC", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
        print_header("Analog inputs", info);
        for (x, idx) in iter {
            print_point("AI", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_frozen_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenAnalogInput, u16)>,
    ) {
        print_header("Frozen analog inputs", info);
        for (x, idx) in iter {
            print_point("FAI", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_analog_input_dead_band(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInputDeadBand, u16)>,
    ) {
        print_header("Analog input deadbands", info);
        for (x, idx) in iter {
            let value = match x {
                AnalogInputDeadBand::U16(v) => f64::from(v),
                AnalogInputDeadBand::U32(v) => f64::from(v),
                AnalogInputDeadBand::F32(v) => f64::from(v),
            };
            println!("AIDB {idx}: value={value}");
        }
    }

    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        print_header("Analog output statuses", info);
        for (x, idx) in iter {
            print_point("AOS", idx, x.value, x.flags, x.time);
        }
    }

    fn handle_binary_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputCommandEvent, u16)>,
    ) {
        print_header("Binary output command events", info);
        for (x, idx) in iter {
            println!(
                "BOCE {idx}: commanded state={} status={:?} time={}",
                x.commanded_state,
                x.status,
                format_time(x.time)
            );
        }
    }

    fn handle_analog_output_command_event(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputCommandEvent, u16)>,
    ) {
        print_header("Analog output command events", info);
        for (x, idx) in iter {
            let value = match x.commanded_value {
                AnalogCommandValue::I16(v) => f64::from(v),
                AnalogCommandValue::I32(v) => f64::from(v),
                AnalogCommandValue::F32(v) => f64::from(v),
                AnalogCommandValue::F64(v) => v,
            };
            println!(
                "AOCE {idx}: commanded value={value} status={:?} time={}",
                x.status,
                format_time(x.time)
            );
        }
    }

    fn handle_unsigned_integer(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (UnsignedInteger, u16)>,
    ) {
        print_header("Unsigned integers", info);
        for (x, idx) in iter {
            println!("UI {idx}: value={}", x.value);
        }
    }

    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &'a mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        print_header("Octet strings", info);
        for (x, idx) in iter {
            println!("OS {idx}: value={x:02X?}");
        }
    }

    fn handle_device_attribute(&mut self, info: HeaderInfo, attr: AnyAttribute) {
        print_header("Device attributes", info);
        match attr {
            AnyAttribute::Known(attr) => match attr {
                KnownAttribute::AttributeList(var, list) => {
                    print_attribute(0, var.variation(), format_variation_list(list))
                }
                KnownAttribute::String(var, value) => print_attribute(0, var.variation(), value),
                KnownAttribute::Float(var, FloatType::F32(value)) => {
                    print_attribute(0, var.variation(), value)
                }
                KnownAttribute::Float(var, FloatType::F64(value)) => {
                    print_attribute(0, var.variation(), value)
                }
                KnownAttribute::UInt(var, value) => print_attribute(0, var.variation(), value),
                KnownAttribute::Bool(var, value) => print_attribute(0, var.variation(), value),
                KnownAttribute::OctetString(var, value) => {
                    print_attribute(0, var.variation(), format!("{value:02X?}"))
                }
                KnownAttribute::DNP3Time(var, value) => {
                    print_attribute(0, var.variation(), value.raw_value())
                }
            },
            AnyAttribute::Other(attr) => {
                let value = match attr.value {
                    AttrValue::VisibleString(v) => v.to_string(),
                    AttrValue::UnsignedInt(v) => v.to_string(),
                    AttrValue::SignedInt(v) => v.to_string(),
                    AttrValue::FloatingPoint(FloatType::F32(v)) => v.to_string(),
                    AttrValue::FloatingPoint(FloatType::F64(v)) => v.to_string(),
                    AttrValue::OctetString(v) | AttrValue::BitString(v) => format!("{v:02X?}"),
                    AttrValue::Dnp3Time(v) => v.raw_value().to_string(),
                    AttrValue::AttrList(list) => format_variation_list(list),
                };
                print_attribute(attr.set.value(), attr.variation, value)
            }
        }
    }

    fn handle_abs_time(&mut self, info: HeaderInfo, time: Timestamp) {
        print_header("Absolute time", info);
        println!("time={}", time.raw_value());
    }
}

fn print_header(name: &str, info: HeaderInfo) {
    println!("{name} ({}, {})", info.qualifier, info.variation);
}

fn print_point(
    name: &str,
    index: u16,
    value: impl std::fmt::Display,
    flags: Flags,
    time: Option<Time>,
) {
    println!(
        "{name} {index}: value={value} flags=0x{:02X} time={}",
        flags.value,
        format_time(time)
    );
}

fn format_time(time: Option<Time>) -> String {
    match time {
        Some(Time::Synchronized(t)) => format!("{} (synchronized)", t.raw_value()),
        Some(Time::Unsynchronized(t)) => format!("{} (unsynchronized)", t.raw_value()),
        None => "none".to_string(),
    }
}

fn format_variation_list(list: VariationList) -> String {
    let items: Vec<String> = list
        .iter()
        .map(|item| {
            let writable = if item.properties.is_writable() {
                " (writable)"
            } else {
                ""
            };
            format!("{}{writable}", item.variation)
        })
        .collect();
    format!("[{}]", items.join(", "))
}

fn print_attribute(set: u8, variation: u8, value: impl std::fmt::Display) {
    println!("attribute set={set} variation={variation} value={value}");
}
// ANCHOR_END: read_handler

// ANCHOR: association_handler
#[derive(Copy, Clone)]
struct ExampleAssociationHandler;

impl AssociationHandler for ExampleAssociationHandler {}
// ANCHOR_END: association_handler

// ANCHOR: association_information
#[derive(Copy, Clone)]
struct ExampleAssociationInformation;

impl AssociationInformation for ExampleAssociationInformation {}
// ANCHOR_END: association_information

// ANCHOR: file_logger
struct FileLogger;

impl FileReader for FileLogger {
    fn opened(&mut self, size: u32) -> FileAction {
        tracing::info!("File opened - size: {size}");
        FileAction::Continue
    }

    fn block_received(&mut self, block_num: u32, data: &[u8]) -> MaybeAsync<FileAction> {
        tracing::info!("Received block {block_num} with size: {}", data.len());
        MaybeAsync::ready(FileAction::Continue)
    }

    fn aborted(&mut self, err: FileError) {
        println!("rf: aborted: {err}");
    }

    fn completed(&mut self) {
        println!("rf: completed");
    }
}
// ANCHOR_END: file_logger

/// Logs each state change of a client connection or serial port
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

fn print_file_info(info: FileInfo) {
    println!("File name: {}", info.name);
    println!("     type: {:?}", info.file_type);
    println!("     size: {}", info.size);
    println!("     created: {}", info.time_created.raw_value());
    println!("     permissions:");
    println!("         world: {}", info.permissions.world);
    println!("         group: {}", info.permissions.group);
    println!("         owner: {}", info.permissions.owner);
}
