//! Example of running a master as a TCP server
use std::collections::HashMap;
use std::net::SocketAddr;

use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec};

use dnp3::decode::*;
use dnp3::link::*;
use dnp3::master::*;
use dnp3::tcp::*;

/// read handler that does nothing
#[derive(Copy, Clone)]
pub struct NullReadHandler;

impl NullReadHandler {
    /// create a boxed instance of the NullReadHandler
    pub fn boxed() -> Box<dyn ReadHandler> {
        Box::new(Self {})
    }
}

impl ReadHandler for NullReadHandler {}

#[derive(Copy, Clone)]
struct NullAssociationHandler;

impl AssociationHandler for NullAssociationHandler {}

#[derive(Copy, Clone)]
struct NullAssociationInformation;

impl AssociationInformation for NullAssociationInformation {}

struct ConnectionHandler {
    channels: HashMap<u16, MasterChannel>,
}

impl ConnectionHandler {
    async fn setup_channel(
        channel: &mut MasterChannel,
        source: u16,
    ) -> Result<AssociationHandle, Box<dyn std::error::Error>> {
        let assoc = channel
            .add_association(
                EndpointAddress::try_new(source)?,
                AssociationConfig::new(
                    EventClasses::all(),
                    EventClasses::all(),
                    Classes::all(),
                    EventClasses::none(),
                ),
                Box::new(NullReadHandler),
                Box::new(NullAssociationHandler),
                Box::new(NullAssociationInformation),
            )
            .await?;
        channel.enable().await?;
        Ok(assoc)
    }
}

impl dnp3::tcp::ConnectionHandler for ConnectionHandler {
    async fn accept(&mut self, _: SocketAddr) -> Result<AcceptAction, Reject> {
        Ok(AcceptAction::GetLinkIdentity)
    }

    async fn start(&mut self, _: MasterChannel, _: SocketAddr) {
        //
    }

    async fn accept_link_id(
        &mut self,
        addr: SocketAddr,
        source: u16,
        _destination: u16,
    ) -> Result<AcceptConfig, Reject> {
        tracing::info!("accepted from {addr:?}:{source}");
        let mut decode_level = DecodeLevel::nothing();
        decode_level.application = AppDecodeLevel::ObjectValues;
        let config = AcceptConfig {
            error_mode: LinkErrorMode::Close,
            config: MasterChannelConfig {
                master_address: EndpointAddress::try_new(1).unwrap(),
                decode_level,
                tx_buffer_size: Default::default(),
                rx_buffer_size: Default::default(),
            },
        };
        Ok(config)
    }

    async fn start_with_link_id(
        &mut self,
        mut channel: MasterChannel,
        _addr: SocketAddr,
        source: u16,
        destination: u16,
    ) {
        tracing::info!("start with source = {source} dest = {destination}");

        match Self::setup_channel(&mut channel, source).await {
            Ok(_) => {
                self.channels.insert(source, channel);
            }
            Err(err) => {
                tracing::warn!("channel setup failed: {err}");
            }
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let _server = spawn_master_tcp_server(
        "127.0.0.1:20000".parse()?,
        LinkIdConfig::default().decode_level(PhysDecodeLevel::Data),
        ConnectionHandler {
            channels: Default::default(),
        },
    )
    .await?;

    let mut reader = FramedRead::new(tokio::io::stdin(), LinesCodec::new());

    loop {
        let cmd = reader.next().await.unwrap()?;
        if cmd == "x" {
            return Ok(());
        }
    }
}
