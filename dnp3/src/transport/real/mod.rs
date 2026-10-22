use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::transport::Transport;

pub(crate) mod assembler;
pub(crate) mod display;
pub(crate) mod header;
pub(crate) mod reader;
pub(crate) mod sequence;
pub(crate) mod writer;

pub(crate) mod constants {
    pub(crate) const FIN_MASK: u8 = 0b1000_0000;
    pub(crate) const FIR_MASK: u8 = 0b0100_0000;
}

/// DNP3 link-layer framing with transport segmentation and reassembly
pub(crate) struct RealTransport;

impl Transport for RealTransport {
    type Reader = reader::Reader;
    type Writer = writer::Writer;

    fn new(
        link_modes: LinkModes,
        role: LinkRole,
        address: EndpointAddress,
        rx_buffer_size: usize,
    ) -> (Self::Reader, Self::Writer) {
        (
            reader::Reader::new(link_modes, role, address, rx_buffer_size),
            writer::Writer::new(role, address),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::decode::DecodeLevel;
    use crate::link::header::{AnyAddress, ControlField};
    use crate::link::test_data;
    use crate::outstation::Feature;
    use crate::transport::{
        FragmentAddr, LinkLayerMessageType, TransportData, TransportRead, TransportWrite,
    };
    use crate::util::phys::{PhysAddr, PhysLayer};

    const MASTER: EndpointAddress = EndpointAddress::raw(1);
    const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);

    // link function codes, including the PRM bit
    const UNCONFIRMED_USER_DATA: u8 = 0x44;
    const REQUEST_LINK_STATUS: u8 = 0x49;
    const LINK_STATUS: u8 = 0x0B;

    fn master() -> (reader::Reader, writer::Writer) {
        RealTransport::new(LinkModes::test(), LinkRole::Master, MASTER, 2048)
    }

    fn outstation(self_address: Feature) -> (reader::Reader, writer::Writer) {
        RealTransport::new(
            LinkModes::test(),
            LinkRole::Outstation { self_address },
            OUTSTATION,
            2048,
        )
    }

    fn link_header(
        from_master: bool,
        function: u8,
        destination: AnyAddress,
        source: EndpointAddress,
    ) -> crate::link::header::Header {
        crate::link::header::Header::new(
            ControlField::from(if from_master { 0x80 } else { 0 } | function),
            destination,
            source.wrap(),
        )
    }

    /// a data frame carrying a whole fragment in one segment
    fn segment(
        from_master: bool,
        destination: AnyAddress,
        source: EndpointAddress,
        seq: u8,
        data: &[u8],
    ) -> Vec<u8> {
        let header = link_header(from_master, UNCONFIRMED_USER_DATA, destination, source);
        test_data::data_frame(header, test_data::segment(true, true, seq), data)
    }

    fn header_only(
        from_master: bool,
        function: u8,
        destination: AnyAddress,
        source: EndpointAddress,
    ) -> Vec<u8> {
        test_data::header_only(link_header(from_master, function, destination, source))
    }

    #[derive(Debug, PartialEq)]
    enum Item {
        Fragment(Vec<u8>),
        LinkStatusRequest(EndpointAddress),
        LinkStatusResponse(EndpointAddress),
    }

    struct Harness {
        reader: reader::Reader,
        writer: writer::Writer,
        io: PhysLayer,
        handle: sfio_tokio_mock_io::Handle,
    }

    impl Harness {
        fn new((reader, writer): (reader::Reader, writer::Writer)) -> Self {
            let (io, handle) = sfio_tokio_mock_io::mock();
            Self {
                reader,
                writer,
                io: PhysLayer::mock(io),
                handle,
            }
        }

        /// queue the frames, then read until something is available
        async fn read(&mut self, frames: &[Vec<u8>]) -> Option<Item> {
            self.handle.read(&frames.concat());
            tokio::time::timeout(
                Duration::from_secs(1),
                self.reader.read(&mut self.io, DecodeLevel::nothing()),
            )
            .await
            .ok()?
            .unwrap();
            self.reader.pop().map(|data| match data {
                TransportData::Fragment(x) => Item::Fragment(x.data.to_vec()),
                TransportData::LinkLayerMessage(x) => match x.message {
                    LinkLayerMessageType::LinkStatusRequest => Item::LinkStatusRequest(x.source),
                    LinkLayerMessageType::LinkStatusResponse => Item::LinkStatusResponse(x.source),
                },
            })
        }

        async fn write(&mut self, destination: EndpointAddress, fragment: &[u8]) {
            let destination = FragmentAddr {
                link: destination,
                phys: PhysAddr::None,
            };
            self.writer
                .write(&mut self.io, DecodeLevel::nothing(), destination, fragment)
                .await
                .unwrap();
        }

        async fn write_link_status_request(&mut self, destination: EndpointAddress) {
            let destination = FragmentAddr {
                link: destination,
                phys: PhysAddr::None,
            };
            self.writer
                .write_link_status_request(&mut self.io, destination, DecodeLevel::nothing())
                .await
                .unwrap();
        }

        /// the bytes written since the last call
        fn writes(&mut self) -> Vec<Vec<u8>> {
            let mut writes = Vec::new();
            while let Some(event) = self.handle.pop_event() {
                if let sfio_tokio_mock_io::Event::Write(x) = event {
                    writes.push(x);
                }
            }
            writes
        }
    }

    #[tokio::test(start_paused = true)]
    async fn master_writes_frames_from_the_master() {
        let mut h = Harness::new(master());
        h.write(OUTSTATION, &[1, 2, 3]).await;
        h.write_link_status_request(OUTSTATION).await;
        assert_eq!(
            h.writes(),
            [
                segment(true, OUTSTATION.wrap(), MASTER, 0, &[1, 2, 3]),
                header_only(true, REQUEST_LINK_STATUS, OUTSTATION.wrap(), MASTER),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_writes_frames_from_the_outstation() {
        for self_address in [Feature::Disabled, Feature::Enabled] {
            let mut h = Harness::new(outstation(self_address));
            h.write(MASTER, &[1, 2, 3]).await;
            h.write_link_status_request(MASTER).await;
            assert_eq!(
                h.writes(),
                [
                    segment(false, MASTER.wrap(), OUTSTATION, 0, &[1, 2, 3]),
                    header_only(false, REQUEST_LINK_STATUS, MASTER.wrap(), OUTSTATION),
                ]
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn master_reads_from_outstations_and_ignores_self_address() {
        let mut h = Harness::new(master());
        let frames = [
            segment(true, MASTER.wrap(), OUTSTATION, 0, &[1]),
            segment(false, AnyAddress::SelfAddress, OUTSTATION, 1, &[2]),
            header_only(
                false,
                REQUEST_LINK_STATUS,
                AnyAddress::SelfAddress,
                OUTSTATION,
            ),
            segment(false, MASTER.wrap(), OUTSTATION, 2, &[3]),
        ];
        assert_eq!(h.read(&frames).await, Some(Item::Fragment(vec![3])));
        assert!(h.writes().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn master_answers_link_status_requests() {
        let mut h = Harness::new(master());
        let request = header_only(false, REQUEST_LINK_STATUS, MASTER.wrap(), OUTSTATION);
        assert_eq!(
            h.read(&[request]).await,
            Some(Item::LinkStatusRequest(OUTSTATION))
        );
        assert_eq!(
            h.writes(),
            [header_only(true, LINK_STATUS, OUTSTATION.wrap(), MASTER)]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn master_reads_link_status_responses() {
        let mut h = Harness::new(master());
        let response = header_only(false, LINK_STATUS, MASTER.wrap(), OUTSTATION);
        assert_eq!(
            h.read(&[response]).await,
            Some(Item::LinkStatusResponse(OUTSTATION))
        );
        assert!(h.writes().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_reads_from_masters_only() {
        for self_address in [Feature::Disabled, Feature::Enabled] {
            let mut h = Harness::new(outstation(self_address));
            let frames = [
                segment(false, OUTSTATION.wrap(), MASTER, 0, &[1]),
                segment(true, OUTSTATION.wrap(), MASTER, 1, &[2]),
            ];
            assert_eq!(h.read(&frames).await, Some(Item::Fragment(vec![2])));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_with_self_address_reads_fragments_sent_to_self_address() {
        let mut h = Harness::new(outstation(Feature::Enabled));
        let frames = [segment(true, AnyAddress::SelfAddress, MASTER, 0, &[1])];
        assert_eq!(h.read(&frames).await, Some(Item::Fragment(vec![1])));
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_with_self_address_answers_link_status_request_from_its_own_address() {
        let mut h = Harness::new(outstation(Feature::Enabled));
        let request = header_only(true, REQUEST_LINK_STATUS, AnyAddress::SelfAddress, MASTER);
        assert_eq!(
            h.read(&[request]).await,
            Some(Item::LinkStatusRequest(MASTER))
        );
        assert_eq!(
            h.writes(),
            [header_only(false, LINK_STATUS, MASTER.wrap(), OUTSTATION)]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn outstation_without_self_address_ignores_frames_sent_to_self_address() {
        let mut h = Harness::new(outstation(Feature::Disabled));
        let frames = [
            segment(true, AnyAddress::SelfAddress, MASTER, 0, &[1]),
            header_only(true, REQUEST_LINK_STATUS, AnyAddress::SelfAddress, MASTER),
            segment(true, OUTSTATION.wrap(), MASTER, 1, &[2]),
        ];
        assert_eq!(h.read(&frames).await, Some(Item::Fragment(vec![2])));
        assert!(h.writes().is_empty());
    }

    // link function codes with the PRM bit, and the FCB and FCV bits of confirmed user data
    const RESET_LINK_STATES: u8 = 0x40;
    const CONFIRMED_USER_DATA_FCB: u8 = 0x73;
    const ACK: u8 = 0x00;

    /// a data frame from the master to the outstation carrying one segment of a fragment
    fn master_segment(fir: bool, fin: bool, seq: u8, data: &[u8]) -> Vec<u8> {
        let header = link_header(true, UNCONFIRMED_USER_DATA, OUTSTATION.wrap(), MASTER);
        test_data::data_frame(header, test_data::segment(fir, fin, seq), data)
    }

    #[tokio::test(start_paused = true)]
    async fn data_frame_without_a_transport_header_is_ignored() {
        let mut h = Harness::new(outstation(Feature::Disabled));
        let frames = [
            master_segment(true, false, 0, &[1]),
            header_only(true, UNCONFIRMED_USER_DATA, OUTSTATION.wrap(), MASTER),
            master_segment(false, true, 1, &[2]),
        ];
        assert_eq!(h.read(&frames).await, Some(Item::Fragment(vec![1, 2])));
        assert!(h.writes().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn confirmed_data_frame_without_a_transport_header_is_acknowledged_and_ignored() {
        let mut h = Harness::new(outstation(Feature::Disabled));
        let frames = [
            header_only(true, RESET_LINK_STATES, OUTSTATION.wrap(), MASTER),
            master_segment(true, false, 0, &[1]),
            header_only(true, CONFIRMED_USER_DATA_FCB, OUTSTATION.wrap(), MASTER),
            master_segment(false, true, 1, &[2]),
        ];
        assert_eq!(h.read(&frames).await, Some(Item::Fragment(vec![1, 2])));
        let ack = header_only(false, ACK, MASTER.wrap(), OUTSTATION);
        assert_eq!(h.writes(), [ack.clone(), ack]);
    }
}
