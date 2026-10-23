// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

use crate::decode::DecodeLevel;
use crate::link::error::LinkError;
use crate::link::header::FrameType;
use crate::link::parser::FramePayload;
use crate::link::reader::LinkModes;
use crate::link::{EndpointAddress, LinkRole};
use crate::transport::real::assembler::{Assembler, Discard};
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
    pub(crate) fn new(
        link_modes: LinkModes,
        role: LinkRole,
        source: EndpointAddress,
        max_rx_buffer: usize,
    ) -> Self {
        Self {
            link: crate::link::layer::Layer::new(link_modes, max_rx_buffer, role, source),
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
        let mut payload = FramePayload::new();

        loop {
            // the reply to the last frame goes out before what that frame carried is returned
            io.finish_reply().await?;

            if self.peek().is_some() {
                return Ok(());
            }

            // the frame is assembled or stored below with nothing awaited, so it can't be lost
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

                        self.assembler.assemble(info, header, data, Discard::log);
                    }
                    [] => tracing::warn!("received link data frame with no payload"),
                },
                FrameType::LinkStatusRequest => {
                    self.pending_link_layer_message = Some(LinkLayerMessage {
                        source: info.source,
                        message: LinkLayerMessageType::LinkStatusRequest,
                    });
                }
                FrameType::LinkStatusResponse => {
                    self.pending_link_layer_message = Some(LinkLayerMessage {
                        source: info.source,
                        message: LinkLayerMessageType::LinkStatusResponse,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::link::test_data::{self, data_frame, header_only};

    const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);
    const MASTER: EndpointAddress = EndpointAddress::raw(1);

    fn segment(fir: bool, fin: bool, seq: u8, data: &[u8]) -> Vec<u8> {
        let header = crate::link::header::Header::unconfirmed_user_data(
            true,
            OUTSTATION.wrap(),
            MASTER.wrap(),
        );
        data_frame(header, test_data::segment(fir, fin, seq), data)
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
                reader: Reader::new(
                    LinkModes::test(),
                    LinkRole::Outstation {
                        self_address: crate::outstation::Feature::Disabled,
                    },
                    OUTSTATION,
                    2048,
                ),
                io: PhysLayer::mock(io),
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

    /// reads dropped while the link layer's reply is waiting on the peer
    mod cancel {
        use std::future::Future;
        use std::pin::pin;
        use std::task::{Context, Waker};

        use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

        use super::*;
        use crate::link::header::ControlField;
        use crate::transport::{FragmentAddr, TransportWrite};
        use crate::util::phys::PhysAddr;

        const FRAGMENT: [u8; 1] = [0xAA];

        // link control fields, including the DIR, PRM and FCV bits
        const RESET_LINK_STATES: u8 = 0xC0;
        const CONFIRMED_USER_DATA: u8 = 0xD3;
        const ACK: u8 = 0x00;
        const LINK_STATUS: u8 = 0x0B;

        fn from_master(control: u8) -> crate::link::header::Header {
            crate::link::header::Header::new(
                ControlField::from(control),
                OUTSTATION.wrap(),
                MASTER.wrap(),
            )
        }

        fn reset_link_states() -> Vec<u8> {
            header_only(from_master(RESET_LINK_STATES))
        }

        fn confirmed(fcb: bool) -> Vec<u8> {
            let mut header = from_master(CONFIRMED_USER_DATA);
            header.control.fcb = fcb;
            data_frame(header, test_data::segment(true, true, 0), &FRAGMENT)
        }

        fn reply(control: u8) -> Vec<u8> {
            header_only(crate::link::header::Header::new(
                ControlField::from(control),
                MASTER.wrap(),
                OUTSTATION.wrap(),
            ))
        }

        fn ack() -> Vec<u8> {
            reply(ACK)
        }

        /// an outstation reader whose writes wait once `capacity` bytes are unread by the peer
        struct Duplex {
            reader: Reader,
            io: PhysLayer,
            peer: DuplexStream,
        }

        impl Duplex {
            fn new(capacity: usize) -> Self {
                let (io, peer) = tokio::io::duplex(capacity);
                Self {
                    reader: Reader::new(
                        LinkModes::test(),
                        LinkRole::Outstation {
                            self_address: crate::outstation::Feature::Disabled,
                        },
                        OUTSTATION,
                        2048,
                    ),
                    io: PhysLayer::duplex(io),
                    peer,
                }
            }

            async fn send(&mut self, frame: &[u8]) {
                self.peer.write_all(frame).await.unwrap();
            }

            /// poll a read once and drop it, returning true if it was still pending
            fn read_and_cancel(&mut self) -> bool {
                let read = pin!(self.reader.read(&mut self.io, DecodeLevel::nothing()));
                read.poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
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

            /// the next `len` bytes the reader wrote
            async fn received(&mut self, len: usize) -> Vec<u8> {
                let mut bytes = vec![0; len];
                self.peer.read_exact(&mut bytes).await.unwrap();
                bytes
            }

            fn nothing_more_received(&mut self) -> bool {
                let mut byte = [0];
                pin!(self.peer.read(&mut byte))
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            }

            fn pop(&mut self) -> Option<Item> {
                self.reader.pop().map(Item::from)
            }

            /// reset the link with `resets` frames, whose ACKs the peer leaves unread, then
            /// cancel the read of confirmed user data while its ACK is waiting on the peer
            async fn cancel_ack(capacity: usize, resets: usize) -> Self {
                let mut h = Self::new(capacity);
                for _ in 0..resets {
                    h.send(&reset_link_states()).await;
                    assert!(h.read_and_cancel());
                }
                h.send(&confirmed(true)).await;
                assert!(h.read_and_cancel());
                h
            }
        }

        /// the peer's buffer holds `capacity` bytes, which the reader fills with `resets` ACKs and
        /// what fits of the ACK to the confirmed user data
        async fn fragment_is_delivered_once_after_cancelling_its_ack(
            capacity: usize,
            resets: usize,
        ) {
            let mut h = Duplex::cancel_ack(capacity, resets).await;
            let acks = ack().repeat(resets.saturating_add(1));
            let (full, rest) = acks.split_at(capacity);

            assert_eq!(h.received(full.len()).await, full);
            assert!(h.read().await);
            assert_eq!(h.received(rest.len()).await, rest);
            assert!(h.nothing_more_received());
            assert_eq!(h.pop(), Some(Item::Fragment(FRAGMENT.to_vec())));
            assert_eq!(h.pop(), None);
        }

        #[tokio::test(start_paused = true)]
        async fn fragment_is_delivered_once_after_cancelling_an_ack_with_nothing_written() {
            fragment_is_delivered_once_after_cancelling_its_ack(20, 2).await;
        }

        #[tokio::test(start_paused = true)]
        async fn fragment_is_delivered_once_after_cancelling_a_partly_written_ack() {
            fragment_is_delivered_once_after_cancelling_its_ack(15, 1).await;
        }

        #[tokio::test(start_paused = true)]
        async fn repeat_of_a_frame_whose_ack_was_cancelled_is_acked_without_redelivery() {
            let mut h = Duplex::cancel_ack(15, 1).await;
            // the master times out waiting for the ACK and repeats the frame
            h.send(&confirmed(true)).await;
            let acks = ack().repeat(3);
            let (full, rest) = acks.split_at(15);

            assert_eq!(h.received(full.len()).await, full);
            assert!(h.read().await);
            assert_eq!(h.pop(), Some(Item::Fragment(FRAGMENT.to_vec())));
            assert!(!h.read().await);
            assert_eq!(h.received(rest.len()).await, rest);
            assert!(h.nothing_more_received());
            assert_eq!(h.pop(), None);
        }

        #[tokio::test(start_paused = true)]
        async fn link_status_request_is_delivered_once_after_cancelling_its_reply() {
            let mut h = Duplex::new(10);
            h.send(&reset_link_states()).await;
            assert!(h.read_and_cancel());
            h.send(&link_status_request()).await;
            assert!(h.read_and_cancel());

            assert_eq!(h.received(10).await, ack());
            assert!(h.read().await);
            assert_eq!(h.received(10).await, reply(LINK_STATUS));
            assert!(h.nothing_more_received());
            assert_eq!(h.pop(), Some(Item::LinkStatusRequest));
            assert_eq!(h.pop(), None);
        }

        #[tokio::test(start_paused = true)]
        async fn writer_finishes_a_cancelled_reply_before_writing() {
            let mut h = Duplex::cancel_ack(15, 1).await;
            let acks = ack().repeat(2);
            let (full, rest) = acks.split_at(15);
            assert_eq!(h.received(full.len()).await, full);

            let mut writer = crate::transport::real::writer::Writer::new(
                LinkRole::Outstation {
                    self_address: crate::outstation::Feature::Disabled,
                },
                OUTSTATION,
            );
            let destination = FragmentAddr {
                link: MASTER,
                phys: PhysAddr::None,
            };
            let response = data_frame(
                crate::link::header::Header::unconfirmed_user_data(
                    false,
                    MASTER.wrap(),
                    OUTSTATION.wrap(),
                ),
                test_data::segment(true, true, 0),
                &FRAGMENT,
            );
            let expected = [rest, &response].concat();
            let mut received = vec![0; expected.len()];
            let (written, read) = tokio::time::timeout(Duration::from_secs(1), async {
                tokio::join!(
                    writer.write(&mut h.io, DecodeLevel::nothing(), destination, &FRAGMENT),
                    h.peer.read_exact(&mut received)
                )
            })
            .await
            .unwrap();
            written.unwrap();
            read.unwrap();

            assert_eq!(received, expected);
            assert!(h.nothing_more_received());
            assert!(h.read().await);
            assert_eq!(h.pop(), Some(Item::Fragment(FRAGMENT.to_vec())));
        }
    }
}
