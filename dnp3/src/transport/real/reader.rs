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
use crate::transport::{LinkLayerMessage, LinkLayerMessageType, TransportData, TransportRead};
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

    pub(crate) fn seed_link(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        self.link.seed(seed_data)
    }
}

impl TransportRead for Reader {
    fn reset(&mut self) {
        self.assembler.reset();
        self.link.reset();
        self.pending_link_layer_message = None;
    }

    fn pop(&mut self) -> Option<TransportData<'_>> {
        if let Some(msg) = self.pending_link_layer_message.take() {
            return Some(TransportData::LinkLayerMessage(msg));
        }

        self.assembler.pop().map(TransportData::Fragment)
    }

    fn peek(&self) -> Option<TransportData<'_>> {
        if let Some(msg) = self.pending_link_layer_message {
            return Some(TransportData::LinkLayerMessage(msg));
        }

        self.assembler.peek().map(TransportData::Fragment)
    }

    async fn read(&mut self, io: &mut PhysLayer, level: DecodeLevel) -> Result<(), LinkError> {
        if self.peek().is_some() {
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use scursor::WriteCursor;

    use super::*;
    use crate::link::constant::MAX_LINK_FRAME_LENGTH;
    use crate::link::format::{format_data_frame, format_header_only, Payload};

    const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);
    const MASTER: EndpointAddress = EndpointAddress::raw(1);

    fn segment(fir: bool, fin: bool, seq: u8, data: &[u8]) -> Vec<u8> {
        let header = crate::link::header::Header::unconfirmed_user_data(
            true,
            OUTSTATION.wrap(),
            MASTER.wrap(),
        );
        let transport = Header::new(
            fin,
            fir,
            crate::transport::real::sequence::Sequence::new(seq),
        );
        let mut buffer = [0; MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        format_data_frame(header, Payload::new(transport.to_u8(), data), &mut cursor)
            .unwrap()
            .frame
            .to_vec()
    }

    fn header_only(header: crate::link::header::Header) -> Vec<u8> {
        let mut buffer = [0; MAX_LINK_FRAME_LENGTH];
        let mut cursor = WriteCursor::new(&mut buffer);
        format_header_only(header, &mut cursor)
            .unwrap()
            .frame
            .to_vec()
    }

    fn link_status_request() -> Vec<u8> {
        header_only(crate::link::header::Header::request_link_status(
            true, OUTSTATION, MASTER,
        ))
    }

    struct Harness {
        reader: Reader,
        io: PhysLayer,
        handle: sfio_tokio_mock_io::Handle,
    }

    impl Harness {
        fn new() -> Self {
            let (io, handle) = sfio_tokio_mock_io::mock();
            Self {
                reader: Reader::outstation(LinkModes::test(), OUTSTATION, Feature::Disabled, 2048),
                io: PhysLayer::Mock(io),
                handle,
            }
        }

        fn queue(&mut self, frames: &[Vec<u8>]) {
            self.handle.read(&frames.concat());
        }

        /// true if `read` completed instead of waiting for more data
        async fn read(&mut self) -> bool {
            tokio::time::timeout(
                Duration::from_secs(1),
                self.reader.read(&mut self.io, DecodeLevel::nothing()),
            )
            .await
            .map(|res| res.unwrap())
            .is_ok()
        }

        fn pop(&mut self) -> Option<Item> {
            self.reader.pop().map(Item::from)
        }

        fn peek(&self) -> Option<Item> {
            self.reader.peek().map(Item::from)
        }
    }

    #[derive(Debug, PartialEq)]
    enum Item {
        Fragment(Vec<u8>),
        LinkStatusRequest,
        LinkStatusResponse,
    }

    impl From<TransportData<'_>> for Item {
        fn from(data: TransportData<'_>) -> Self {
            match data {
                TransportData::Fragment(fragment) => Item::Fragment(fragment.data.to_vec()),
                TransportData::LinkLayerMessage(msg) => match msg.message {
                    LinkLayerMessageType::LinkStatusRequest => Item::LinkStatusRequest,
                    LinkLayerMessageType::LinkStatusResponse => Item::LinkStatusResponse,
                },
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn returns_a_fragment_assembled_from_several_segments() {
        let mut h = Harness::new();
        h.queue(&[
            segment(true, false, 0, &[1, 2]),
            segment(false, true, 1, &[3]),
        ]);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![1, 2, 3])));
    }

    #[tokio::test(start_paused = true)]
    async fn read_returns_at_once_while_a_fragment_is_pending() {
        let mut h = Harness::new();
        h.queue(&[segment(true, true, 0, &[1]), segment(true, true, 1, &[2])]);
        assert!(h.read().await);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![1])));
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![2])));
    }

    #[tokio::test(start_paused = true)]
    async fn read_returns_at_once_while_a_link_layer_message_is_pending() {
        let mut h = Harness::new();
        h.queue(&[link_status_request()]);
        assert!(h.read().await);
        assert_eq!(h.peek(), Some(Item::LinkStatusRequest));
        // nothing more to read, so a read that doesn't return at once times out
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::LinkStatusRequest));
        assert_eq!(h.pop(), None);
    }

    #[tokio::test(start_paused = true)]
    async fn pending_link_layer_message_is_not_overwritten_by_the_next() {
        let mut h = Harness::new();
        // DIR set, secondary function 11: link status
        let link_status = header_only(crate::link::header::Header::new(
            crate::link::header::ControlField::from(0x8B),
            OUTSTATION.wrap(),
            MASTER.wrap(),
        ));
        h.queue(&[link_status_request(), link_status]);
        assert!(h.read().await);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::LinkStatusRequest));
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::LinkStatusResponse));
    }

    #[tokio::test(start_paused = true)]
    async fn link_layer_message_during_assembly_leaves_the_series() {
        let mut h = Harness::new();
        h.queue(&[
            segment(true, false, 0, &[1]),
            link_status_request(),
            segment(false, true, 1, &[2]),
        ]);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::LinkStatusRequest));
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![1, 2])));
    }

    #[tokio::test(start_paused = true)]
    async fn segment_with_only_a_transport_header_is_not_passed_up() {
        let mut h = Harness::new();
        h.queue(&[segment(true, true, 0, &[]), segment(true, true, 1, &[1])]);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![1])));
    }

    #[tokio::test(start_paused = true)]
    async fn data_frame_without_a_transport_header_is_ignored() {
        let mut h = Harness::new();
        h.queue(&[
            header_only(crate::link::header::Header::unconfirmed_user_data(
                true,
                OUTSTATION.wrap(),
                MASTER.wrap(),
            )),
            segment(true, true, 0, &[1]),
        ]);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![1])));
    }

    #[tokio::test(start_paused = true)]
    async fn reset_discards_pending_input() {
        let mut h = Harness::new();
        h.queue(&[segment(true, false, 0, &[1]), link_status_request()]);
        assert!(h.read().await);
        h.reader.reset();
        assert_eq!(h.pop(), None);
        h.queue(&[segment(false, true, 1, &[2]), segment(true, true, 2, &[3])]);
        assert!(h.read().await);
        assert_eq!(h.pop(), Some(Item::Fragment(vec![3])));
    }
}
