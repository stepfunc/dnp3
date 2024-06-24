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
use dnp3::tcp::*;

#[cfg(feature = "serial")]
use dnp3::serial::*;
#[cfg(feature = "tls")]
use dnp3::tcp::tls::*;

use dnp3::outstation::FreezeInterval;
use dnp3::udp::spawn_master_udp;
use std::process::exit;

/// read handler that does nothing
#[derive(Copy, Clone)]
pub struct ExampleReadHandler;

impl ExampleReadHandler {
    /// create a boxed instance of the NullReadHandler
    pub fn boxed() -> Box<dyn ReadHandler> {
        Box::new(Self {})
    }
}

// ANCHOR: read_handler
impl ReadHandler for ExampleReadHandler {
    fn begin_fragment(&mut self, _read_type: ReadType, header: ResponseHeader) -> MaybeAsync<()> {
        println!(
            "Beginning fragment (broadcast: {})",
            header.iin.iin1.get_broadcast()
        );
        MaybeAsync::ready(())
    }

    fn end_fragment(&mut self, _read_type: ReadType, _header: ResponseHeader) -> MaybeAsync<()> {
        println!("End fragment");
        MaybeAsync::ready(())
    }

    fn handle_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryInput, u16)>,
    ) {
        println!("Binary Inputs:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "BI {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_double_bit_binary_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (DoubleBitBinaryInput, u16)>,
    ) {
        println!("Double Bit Binary Inputs:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "DBBI {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_binary_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (BinaryOutputStatus, u16)>,
    ) {
        println!("Binary Output Statuses:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "BOS {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_counter(&mut self, info: HeaderInfo, iter: &mut dyn Iterator<Item = (Counter, u16)>) {
        println!("Counters:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "Counter {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_frozen_counter(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (FrozenCounter, u16)>,
    ) {
        println!("Frozen Counters:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "Frozen Counter {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_analog_input(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogInput, u16)>,
    ) {
        println!("Analog Inputs:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "AI {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_analog_output_status(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (AnalogOutputStatus, u16)>,
    ) {
        println!("Analog Output Statuses:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!(
                "AOS {}: Value={} Flags={:#04X} Time={:?}",
                idx, x.value, x.flags.value, x.time
            );
        }
    }

    fn handle_octet_string(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (&[u8], u16)>,
    ) {
        println!("Octet Strings:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!("Octet String {}: Value={:X?}", idx, x);
        }
    }

    fn handle_device_attribute(&mut self, _info: HeaderInfo, attr: AnyAttribute) {
        println!("Device attribute: {:?}", attr)
    }
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
        tracing::info!("File transfer aborted: {}", err);
    }

    fn completed(&mut self) {
        tracing::info!("File transfer completed");
    }
}
// ANCHOR_END: file_logger

/*
  Example program using the master API from within the Tokio runtime.

  The program initializes a master channel based on the command line argument and then enters a loop
  reading console input allowing the user to perform common tasks interactively.

  All the configuration values are hard-coded but can be changed with a recompile.
*/
// ANCHOR: runtime_init
#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ANCHOR_END: runtime_init

    // ANCHOR: logging
    // initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    // ANCHOR_END: logging

    // spawn the master channel based on the command line argument
    let (mut channel, mut association) = create_channel_and_association().await?;

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

    loop {
        let cmd = reader.next().await.unwrap()?;
        if cmd == "x" {
            return Ok(());
        } else if let Err(err) = handler.run_one_command(&cmd).await {
            tracing::error!("Error: {err}");
        }
    }
}

struct CliHandler {
    poll: PollHandle,
    channel: MasterChannel,
    association: AssociationHandle,
}

impl CliHandler {
    async fn run_one_command(&mut self, cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
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
            "lsr" => {
                tracing::info!("{:?}", self.association.check_link_status().await);
            }
            s => println!("unknown command: {}", s),
        }
        Ok(())
    }
}

// create the specified channel based on the command line argument
async fn create_channel_and_association(
) -> Result<(MasterChannel, AssociationHandle), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let transport: &str = match args.as_slice() {
        [_, x] => x,
        _ => {
            eprintln!("please specify a transport:");
            eprintln!("usage: master <transport> (tcp, udp, serial, tls-ca, tls-self-signed)");
            exit(-1);
        }
    };
    match transport {
        "tcp" => {
            let mut channel = create_tcp_channel()?;
            let assoc = add_association(&mut channel).await?;
            Ok((channel, assoc))
        }
        "udp" => {
            let mut channel = create_udp_channel()?;
            let assoc = add_udp_association(&mut channel).await?;
            Ok((channel, assoc))
        }
        #[cfg(feature = "serial")]
        "serial" => {
            let mut channel = create_serial_channel()?;
            let assoc = add_association(&mut channel).await?;
            Ok((channel, assoc))
        }
        #[cfg(feature = "tls")]
        "tls-ca" => {
            let mut channel = create_tls_channel(get_tls_authority_config()?)?;
            let assoc = add_association(&mut channel).await?;
            Ok((channel, assoc))
        }
        #[cfg(feature = "tls")]
        "tls-self-signed" => {
            let mut channel = create_tls_channel(get_tls_self_signed_config()?)?;
            let assoc = add_association(&mut channel).await?;
            Ok((channel, assoc))
        }
        _ => {
            eprintln!(
                "unknown transport '{}', options are (tcp, serial, tls-ca, tls-self-signed)",
                transport
            );
            exit(-1);
        }
    }
}

async fn add_association(
    channel: &mut MasterChannel,
) -> Result<AssociationHandle, Box<dyn std::error::Error>> {
    // ANCHOR: association_create
    let association = channel
        .add_association(
            EndpointAddress::try_new(1024)?,
            get_association_config(),
            ExampleReadHandler::boxed(),
            Box::new(ExampleAssociationHandler),
            Box::new(ExampleAssociationInformation),
        )
        .await?;
    // ANCHOR_END: association_create
    Ok(association)
}

async fn add_udp_association(
    channel: &mut MasterChannel,
) -> Result<AssociationHandle, Box<dyn std::error::Error>> {
    // ANCHOR: association_create_udp
    let association = channel
        .add_udp_association(
            EndpointAddress::try_new(1024)?,
            "127.0.0.1:20000".parse()?,
            get_association_config(),
            ExampleReadHandler::boxed(),
            Box::new(ExampleAssociationHandler),
            Box::new(ExampleAssociationInformation),
        )
        .await?;
    // ANCHOR_END: association_create_udp
    Ok(association)
}

// ANCHOR: master_channel_config
fn get_master_channel_config() -> Result<MasterChannelConfig, Box<dyn std::error::Error>> {
    let mut config = MasterChannelConfig::new(EndpointAddress::try_new(1)?);
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

#[cfg(feature = "tls")]
fn get_tls_self_signed_config() -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_self_signed_config
    let config = TlsClientConfig::self_signed(
        Path::new("./certs/self_signed/entity2_cert.pem"),
        Path::new("./certs/self_signed/entity1_cert.pem"),
        Path::new("./certs/self_signed/entity1_key.pem"),
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_self_signed_config
    Ok(config)
}

#[cfg(feature = "tls")]
fn get_tls_authority_config() -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    use std::path::Path;
    // ANCHOR: tls_ca_chain_config
    let config = TlsClientConfig::full_pki(
        Some("test.com".to_string()),
        Path::new("./certs/ca_chain/ca_cert.pem"),
        Path::new("./certs/ca_chain/entity1_cert.pem"),
        Path::new("./certs/ca_chain/entity1_key.pem"),
        None, // no password
        MinTlsVersion::V12,
    )?;
    // ANCHOR_END: tls_ca_chain_config
    Ok(config)
}

fn create_tcp_channel() -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_master_tcp_channel
    let channel = spawn_master_tcp_client(
        LinkErrorMode::Close,
        get_master_channel_config()?,
        EndpointList::new("127.0.0.1:20000".to_owned(), &[]),
        ConnectStrategy::default(),
        NullListener::create(),
    );
    // ANCHOR_END: create_master_tcp_channel
    Ok(channel)
}

fn create_udp_channel() -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_master_udp_channel
    let channel = spawn_master_udp(
        "127.0.0.1:20001".parse()?,
        LinkReadMode::Datagram,
        Timeout::from_secs(5)?,
        get_master_channel_config()?,
    );
    // ANCHOR_END: create_master_udp_channel
    Ok(channel)
}

#[cfg(feature = "serial")]
fn create_serial_channel() -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_master_serial_channel
    let channel = spawn_master_serial(
        get_master_channel_config()?,
        "/dev/ttySIM0", // change this for your system
        SerialSettings::default(),
        Duration::from_secs(1),
        NullListener::create(),
    );
    // ANCHOR_END: create_master_serial_channel
    Ok(channel)
}

#[cfg(feature = "tls")]
fn create_tls_channel(
    tls_config: TlsClientConfig,
) -> Result<MasterChannel, Box<dyn std::error::Error>> {
    // ANCHOR: create_master_tls_channel
    let channel = spawn_master_tls_client(
        LinkErrorMode::Close,
        get_master_channel_config()?,
        EndpointList::new("127.0.0.1:20001".to_owned(), &[]),
        ConnectStrategy::default(),
        NullListener::create(),
        tls_config,
    );
    // ANCHOR_END: create_master_tls_channel
    Ok(channel)
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
