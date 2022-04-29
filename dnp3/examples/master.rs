use std::time::Duration;

use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

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

use std::path::Path;
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

    fn handle_octet_string<'a>(
        &mut self,
        info: HeaderInfo,
        iter: &mut dyn Iterator<Item = (&'a [u8], u16)>,
    ) {
        println!("Octet Strings:");
        println!("Qualifier: {}", info.qualifier);
        println!("Variation: {}", info.variation);

        for (x, idx) in iter {
            println!("Octet String {}: Value={:X?}", idx, x);
        }
    }
}
// ANCHOR_END: read_handler

// ANCHOR: association_handler
#[derive(Copy, Clone)]
pub struct ExampleAssociationHandler;

impl AssociationHandler for ExampleAssociationHandler {}
// ANCHOR_END: association_handler

// ANCHOR: association_information
#[derive(Copy, Clone)]
pub struct ExampleAssociationInformation;

impl AssociationInformation for ExampleAssociationInformation {}
// ANCHOR_END: association_information

/*
  Example program using the master API from within the Tokio runtime.

  The program initializes a master channel based on the command line argument and then enters a loop
  reading console input allowing the user to perform common tasks interactively.

  All of the configuration values are hard-coded but can be changed with a recompile.
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
    let mut channel = create_channel()?;

    // ANCHOR: association_create
    let mut association = channel
        .add_association(
            EndpointAddress::try_new(1024)?,
            get_association_config(),
            ExampleReadHandler::boxed(),
            Box::new(ExampleAssociationHandler),
            Box::new(ExampleAssociationInformation),
        )
        .await?;
    // ANCHOR_END: association_create

    // create an event poll
    // ANCHOR: add_poll
    let mut poll = association
        .add_poll(
            ReadRequest::ClassScan(Classes::class123()),
            Duration::from_secs(5),
        )
        .await?;
    // ANCHOR_END: add_poll

    // enable communications
    channel.enable().await?;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        match reader.next().await.unwrap()?.as_str() {
            "x" => return Ok(()),
            "enable" => {
                channel.enable().await?;
            }
            "disable" => {
                channel.disable().await?;
            }
            "dln" => {
                channel.set_decode_level(DecodeLevel::nothing()).await?;
            }
            "dlv" => {
                channel
                    .set_decode_level(AppDecodeLevel::ObjectValues.into())
                    .await?;
            }
            "rao" => {
                if let Err(err) = association
                    .read(ReadRequest::all_objects(Variation::Group40Var0))
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "rmo" => {
                if let Err(err) = association
                    .read(ReadRequest::multiple_headers(&[
                        ReadHeader::all_objects(Variation::Group10Var0),
                        ReadHeader::all_objects(Variation::Group40Var0),
                    ]))
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "cmd" => {
                // ANCHOR: assoc_control
                if let Err(err) = association
                    .operate(
                        CommandMode::SelectBeforeOperate,
                        CommandBuilder::single_header_u16(
                            Group12Var1::from_op_type(OpType::LatchOn),
                            3u16,
                        ),
                    )
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
                // ANCHOR_END: assoc_control
            }
            "evt" => poll.demand().await?,
            "lts" => {
                if let Err(err) = association.synchronize_time(TimeSyncProcedure::Lan).await {
                    tracing::warn!("error: {}", err);
                }
            }
            "nts" => {
                if let Err(err) = association
                    .synchronize_time(TimeSyncProcedure::NonLan)
                    .await
                {
                    tracing::warn!("error: {}", err);
                }
            }
            "crt" => {
                let result = association.cold_restart().await;

                match result {
                    Ok(delay) => tracing::info!("restart delay: {:?}", delay),
                    Err(err) => tracing::warn!("error: {}", err),
                }
            }
            "wrt" => {
                let result = association.warm_restart().await;

                match result {
                    Ok(delay) => tracing::info!("restart delay: {:?}", delay),
                    Err(err) => tracing::warn!("error: {}", err),
                }
            }
            "lsr" => {
                tracing::info!("{:?}", association.check_link_status().await);
            }
            s => println!("unknown command: {}", s),
        }
    }
}

// create the specified channel based on the command line argument
fn create_channel() -> Result<MasterChannel, Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let transport: &str = match args.as_slice() {
        [_, x] => x,
        _ => {
            eprintln!("please specify a transport:");
            eprintln!("usage: master <transport> (tcp, serial, tls-ca, tls-self-signed)");
            exit(-1);
        }
    };
    match transport {
        "tcp" => create_tcp_channel(),
        #[cfg(feature = "serial")]
        "serial" => create_serial_channel(),
        #[cfg(feature = "tls")]
        "tls-ca" => create_tls_channel(get_tls_authority_config()?),
        #[cfg(feature = "tls")]
        "tls-self-signed" => create_tls_channel(get_tls_self_signed_config()?),
        _ => {
            eprintln!(
                "unknown transport '{}', options are (tcp, serial, tls-ca, tls-self-signed)",
                transport
            );
            exit(-1);
        }
    }
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
    // ANCHOR: tls_self_signed_config
    let config = TlsClientConfig::new(
        "test.com",
        &Path::new("./certs/self_signed/entity2_cert.pem"),
        &Path::new("./certs/self_signed/entity1_cert.pem"),
        &Path::new("./certs/self_signed/entity1_key.pem"),
        None, // no password
        MinTlsVersion::V12,
        CertificateMode::SelfSigned,
    )?;
    // ANCHOR_END: tls_self_signed_config
    Ok(config)
}

#[cfg(feature = "tls")]
fn get_tls_authority_config() -> Result<TlsClientConfig, Box<dyn std::error::Error>> {
    // ANCHOR: tls_ca_chain_config
    let config = TlsClientConfig::new(
        "test.com",
        &Path::new("./certs/ca_chain/ca_cert.pem"),
        &Path::new("./certs/ca_chain/entity1_cert.pem"),
        &Path::new("./certs/ca_chain/entity1_key.pem"),
        None, // no password
        MinTlsVersion::V12,
        CertificateMode::AuthorityBased,
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
