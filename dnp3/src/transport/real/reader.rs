use crate::app::EndpointType;
use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::FrameType;
use crate::link::parser::FramePayload;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;
use crate::transport::real::assembler::{Assembler, AssemblyState};
use crate::transport::real::display::SegmentDisplay;
use crate::transport::real::header::Header;
use crate::transport::{LinkLayerMessage, LinkLayerMessageType, TransportData};
use crate::util::phys::PhysLayer;

pub(crate) struct Reader {
    link: crate::link::layer::Layer,
    assembler: Assembler,
    pending_link_layer_message: Option<LinkLayerMessage>,
}

impl Reader {
    pub(crate) fn master(
        link_modes: LinkModes,
        source: EndpointAddress,
        max_rx_buffer: usize,
    ) -> Self {
        Self {
            link: crate::link::layer::Layer::new(
                link_modes,
                max_rx_buffer,
                EndpointType::Master,
                Feature::Disabled,
                source,
            ),
            assembler: Assembler::new(max_rx_buffer),
            pending_link_layer_message: None,
        }
    }

    pub(crate) fn outstation(
        link_modes: LinkModes,
        source: EndpointAddress,
        self_address: Feature,
        max_rx_buffer: usize,
    ) -> Self {
        Self {
            link: crate::link::layer::Layer::new(
                link_modes,
                max_rx_buffer,
                EndpointType::Outstation,
                self_address,
                source,
            ),
            assembler: Assembler::new(max_rx_buffer),
            pending_link_layer_message: None,
        }
    }

    pub(crate) fn reset(&mut self) {
        self.assembler.reset();
        self.link.reset();
        self.pending_link_layer_message = None;
    }

    pub(crate) fn pop(&mut self) -> Option<TransportData> {
        if let Some(msg) = self.pending_link_layer_message.take() {
            return Some(TransportData::LinkLayerMessage(msg));
        }

        self.assembler.pop().map(TransportData::Fragment)
    }

    pub(crate) fn peek(&self) -> Option<TransportData> {
        if let Some(msg) = self.pending_link_layer_message {
            return Some(TransportData::LinkLayerMessage(msg));
        }

        self.assembler.peek().map(TransportData::Fragment)
    }

    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), scursor::WriteError> {
        self.link.seed(seed_data)
    }

    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
    ) -> Result<(), LinkError> {
        if self.assembler.peek().is_some() {
            return Ok(());
        }

        let mut payload = FramePayload::new();

        loop {
            let info = self.link.read(io, level, &mut payload).await?;

            match info.frame_type {
                FrameType::Data => match payload.get() {
                    [transport, data @ ..] => {
                        let header = Header::from_u8(*transport);
                        if level.transport.enabled() {
                            tracing::info!(
                                "TRANSPORT RX - {}",
                                SegmentDisplay::new(header, data, level.transport)
                            );
                        }

                        if let AssemblyState::Complete = self.assembler.assemble(info, header, data)
                        {
                            return Ok(());
                        }
                    }
                    [] => tracing::warn!("received link data frame with no payload"),
                },
                FrameType::LinkStatusRequest => {
                    self.pending_link_layer_message = Some(LinkLayerMessage {
                        source: info.source,
                        message: LinkLayerMessageType::LinkStatusRequest,
                    });
                    return Ok(());
                }
                FrameType::LinkStatusResponse => {
                    self.pending_link_layer_message = Some(LinkLayerMessage {
                        source: info.source,
                        message: LinkLayerMessageType::LinkStatusResponse,
                    });
                    return Ok(());
                }
            }
        }
    }
}
