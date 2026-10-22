use crate::app::EndpointType;
use crate::decode::DecodeLevel;
use crate::link::display::LinkDisplay;
use crate::link::error::LinkError;
use crate::link::format::header_bytes;
use crate::link::function::Function;
use crate::link::header::{
    AnyAddress, BroadcastConfirmMode, ControlField, FrameInfo, FrameType, Header,
};
use crate::link::parser::FramePayload;
use crate::link::reader::LinkModes;
use crate::link::EndpointAddress;
use crate::outstation::Feature;
use crate::util::phys::{PhysAddr, PhysLayer};

enum SecondaryState {
    NotReset,
    Reset(bool), // the next expected fcb
}

pub(crate) struct Layer {
    endpoint_type: EndpointType,
    self_address: Feature,
    local_address: EndpointAddress,
    secondary_state: SecondaryState,
    reader: super::reader::Reader,
    tx_buffer: [u8; super::constant::LINK_HEADER_LENGTH],
}

pub(crate) struct Reply {
    address: EndpointAddress,
    function: Function,
}

/// Why the link layer ignored a frame
#[derive(Copy, Clone, Debug)]
pub(crate) enum Ignored {
    UnknownSource(AnyAddress),
    OtherDestination(EndpointAddress),
    SelfAddress,
    ReservedDestination(u16),
    BroadcastToMaster,
    BroadcastFunction(Function),
    UnconfirmedWithFcv,
    ResetWithFcv,
    ConfirmedWithoutFcv,
    /// confirmed user data before the link was reset
    NotReset,
    /// confirmed user data that repeats the previous frame's FCB
    RepeatedFcb,
    LinkStatusWithFcv,
    Function(Function),
}

impl Ignored {
    /// log the reason, which [`Layer::read`] passes to [`Layer::process_header`]
    pub(crate) fn log(self) {
        match self {
            Self::NotReset | Self::RepeatedFcb => tracing::info!("{self}"),
            _ => tracing::warn!("{self}"),
        }
    }
}

impl std::fmt::Display for Ignored {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownSource(x) => write!(f, "ignoring frame from unknown source address: {x}"),
            Self::OtherDestination(x) => write!(f, "ignoring frame sent to address: {x}"),
            Self::SelfAddress => f.write_str("ignoring frame sent to self address"),
            Self::ReservedDestination(x) => {
                write!(f, "ignoring frame sent to reserved address: {x}")
            }
            Self::BroadcastToMaster => f.write_str("ignoring broadcast frame sent to master"),
            Self::BroadcastFunction(x) => {
                write!(f, "ignoring broadcast frame with function: {x:?}")
            }
            Self::UnconfirmedWithFcv => {
                f.write_str("ignoring unconfirmed user data with FCV bit set")
            }
            Self::ResetWithFcv => {
                f.write_str("ignoring reset link states request with FCV bit set")
            }
            Self::ConfirmedWithoutFcv => {
                f.write_str("ignoring confirmed user data with FCV bit NOT set")
            }
            Self::NotReset => {
                f.write_str("ignoring confirmed user data while secondary state is not reset")
            }
            Self::RepeatedFcb => f.write_str("ignoring confirmed user data with non-matching fcb"),
            Self::LinkStatusWithFcv => f.write_str("ignoring request link status with FCV bit set"),
            Self::Function(x) => write!(f, "ignoring frame with function code: {x:?}"),
        }
    }
}

impl Reply {
    fn new(address: EndpointAddress, function: Function) -> Self {
        Self { address, function }
    }
}

impl Layer {
    pub(crate) fn new(
        link_modes: LinkModes,
        max_rx_fragment_size: usize,
        endpoint_type: EndpointType,
        self_address: Feature,
        local_address: EndpointAddress,
    ) -> Self {
        Self {
            endpoint_type,
            self_address,
            local_address,
            secondary_state: SecondaryState::NotReset,
            reader: super::reader::Reader::new(link_modes, max_rx_fragment_size),
            tx_buffer: [0; super::constant::LINK_HEADER_LENGTH],
        }
    }

    pub(crate) fn seed(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        self.reader.seed(seed_data)
    }

    pub(crate) fn reset(&mut self) {
        self.secondary_state = SecondaryState::NotReset;
        self.reader.reset();
    }

    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        payload: &mut FramePayload,
    ) -> Result<FrameInfo, LinkError> {
        loop {
            if let Some(address) = self.read_one(io, level, payload).await? {
                return Ok(address);
            }
        }
    }

    pub(crate) fn get_header(&self, reply: Reply) -> Header {
        Header::new(
            ControlField::new(self.endpoint_type.dir_bit(), reply.function),
            reply.address.wrap(),
            self.local_address.wrap(),
        )
    }

    pub(crate) fn format_reply(&mut self, header: Header) -> &[u8] {
        self.tx_buffer = header_bytes(header, super::constant::MIN_HEADER_LENGTH_VALUE);
        &self.tx_buffer
    }

    async fn read_one(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        payload: &mut FramePayload,
    ) -> Result<Option<FrameInfo>, LinkError> {
        let (header, addr) = self.reader.read_frame(io, payload, level).await?;
        let (info, reply) = self.process_header(header, addr, Ignored::log);
        if let Some(reply) = reply {
            let header = self.get_header(reply);
            if level.link.enabled() {
                tracing::info!("LINK TX - {}", LinkDisplay::new(header, &[], level.link));
            }
            io.write(self.format_reply(header), addr, level.physical)
                .await?
        }
        Ok(info)
    }

    /// Process a frame's header, passing the reason to `ignore` whenever the frame is ignored
    pub(crate) fn process_header<F: FnMut(Ignored)>(
        &mut self,
        header: Header,
        addr: PhysAddr,
        mut ignore: F,
    ) -> (Option<FrameInfo>, Option<Reply>) {
        // ignore frames sent from the same endpoint type
        if header.control.master == self.endpoint_type.dir_bit() {
            //  we don't log this
            return (None, None);
        }

        // validate the source address
        let source: EndpointAddress = match header.source {
            AnyAddress::Endpoint(x) => x,
            AnyAddress::Reserved(_) | AnyAddress::Broadcast(_) | AnyAddress::SelfAddress => {
                ignore(Ignored::UnknownSource(header.source));
                return (None, None);
            }
        };

        // validate the destination address
        let broadcast: Option<BroadcastConfirmMode> = match header.destination {
            AnyAddress::Endpoint(x) => {
                if x == self.local_address {
                    None
                } else {
                    ignore(Ignored::OtherDestination(x));
                    return (None, None);
                }
            }
            AnyAddress::SelfAddress => {
                if self.self_address.is_enabled() {
                    // just pretend like it was sent to us
                    None
                } else {
                    ignore(Ignored::SelfAddress);
                    return (None, None);
                }
            }
            AnyAddress::Reserved(x) => {
                ignore(Ignored::ReservedDestination(x));
                return (None, None);
            }
            AnyAddress::Broadcast(mode) => match self.endpoint_type {
                EndpointType::Master => {
                    ignore(Ignored::BroadcastToMaster);
                    return (None, None);
                }
                EndpointType::Outstation => Some(mode),
            },
        };

        // broadcasts may only accept user data
        if broadcast.is_some()
            && !matches!(
                header.control.func,
                Function::PriUnconfirmedUserData | Function::PriConfirmedUserData
            )
        {
            ignore(Ignored::BroadcastFunction(header.control.func));
            return (None, None);
        }

        match header.control.func {
            Function::PriUnconfirmedUserData => {
                if header.control.fcv {
                    ignore(Ignored::UnconfirmedWithFcv);
                    return (None, None);
                }

                (
                    Some(FrameInfo::new(source, broadcast, FrameType::Data, addr)),
                    None,
                )
            }
            Function::PriResetLinkStates => {
                if header.control.fcv {
                    ignore(Ignored::ResetWithFcv);
                    return (None, None);
                }

                self.secondary_state = SecondaryState::Reset(true);
                (None, Some(Reply::new(source, Function::SecAck)))
            }
            Function::PriConfirmedUserData => {
                if !header.control.fcv {
                    ignore(Ignored::ConfirmedWithoutFcv);
                    return (None, None);
                }

                match self.secondary_state {
                    SecondaryState::NotReset => {
                        ignore(Ignored::NotReset);
                        (None, None)
                    }
                    SecondaryState::Reset(expected) => {
                        let response = if broadcast.is_none() {
                            Some(Reply::new(source, Function::SecAck))
                        } else {
                            // Do not answer if it's a broadcast
                            None
                        };

                        if header.control.fcb == expected {
                            self.secondary_state = SecondaryState::Reset(!expected);
                            (
                                Some(FrameInfo::new(source, broadcast, FrameType::Data, addr)),
                                response,
                            )
                        } else {
                            ignore(Ignored::RepeatedFcb);
                            (None, response)
                        }
                    }
                }
            }
            Function::PriRequestLinkStatus => {
                if header.control.fcv {
                    ignore(Ignored::LinkStatusWithFcv);
                    return (None, None);
                }

                (
                    Some(FrameInfo::new(
                        source,
                        broadcast,
                        FrameType::LinkStatusRequest,
                        addr,
                    )),
                    Some(Reply::new(source, Function::SecLinkStatus)),
                )
            }
            Function::SecLinkStatus => (
                Some(FrameInfo::new(
                    source,
                    broadcast,
                    FrameType::LinkStatusResponse,
                    addr,
                )),
                None,
            ),
            function @ (Function::PriTestLinkStates
            | Function::SecAck
            | Function::SecNack
            | Function::SecNotSupported
            | Function::Unknown(_)) => {
                ignore(Ignored::Function(function));
                (None, None)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const MASTER: EndpointAddress = EndpointAddress::raw(1);
    const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);

    const LINK_STATUS_FROM_OUTSTATION: [u8; 10] =
        [0x05, 0x64, 0x05, 0x0B, 0x01, 0x00, 0x00, 0x04, 0x64, 0x40];
    const ACK_FROM_OUTSTATION: [u8; 10] =
        [0x05, 0x64, 0x05, 0x00, 0x01, 0x00, 0x00, 0x04, 0x27, 0x70];
    const LINK_STATUS_FROM_MASTER: [u8; 10] =
        [0x05, 0x64, 0x05, 0x8B, 0x00, 0x04, 0x01, 0x00, 0x2E, 0xF7];
    const ACK_FROM_MASTER: [u8; 10] = [0x05, 0x64, 0x05, 0x80, 0x00, 0x04, 0x01, 0x00, 0x6D, 0xC7];

    const BROADCASTS: [BroadcastConfirmMode; 3] = [
        BroadcastConfirmMode::Optional,
        BroadcastConfirmMode::Mandatory,
        BroadcastConfirmMode::NotRequired,
    ];

    fn master() -> Layer {
        Layer::new(
            LinkModes::test(),
            2048,
            EndpointType::Master,
            Feature::Disabled,
            MASTER,
        )
    }

    fn outstation(self_address: Feature) -> Layer {
        Layer::new(
            LinkModes::test(),
            2048,
            EndpointType::Outstation,
            self_address,
            OUTSTATION,
        )
    }

    /// a frame header from the master (`from_master`) or the outstation, sent to `destination`
    fn header(from_master: bool, function: Function, destination: AnyAddress) -> Header {
        let source = if from_master { MASTER } else { OUTSTATION };
        Header::new(
            ControlField::new(from_master, function),
            destination,
            source.wrap(),
        )
    }

    fn confirmed(from_master: bool, fcb: bool, destination: AnyAddress) -> Header {
        let mut header = header(from_master, Function::PriConfirmedUserData, destination);
        header.control.fcv = true;
        header.control.fcb = fcb;
        header
    }

    struct Processed {
        info: Option<FrameInfo>,
        reply: Option<Vec<u8>>,
        ignored: Vec<Ignored>,
    }

    impl Processed {
        fn is_silent(&self) -> bool {
            self.info.is_none() && self.reply.is_none() && self.ignored.is_empty()
        }
    }

    fn process(layer: &mut Layer, header: Header) -> Processed {
        let mut ignored = Vec::new();
        let (info, reply) = layer.process_header(header, PhysAddr::None, |x| ignored.push(x));
        let reply = reply.map(|reply| {
            let header = layer.get_header(reply);
            layer.format_reply(header).to_vec()
        });
        Processed {
            info,
            reply,
            ignored,
        }
    }

    fn data_from(source: EndpointAddress, broadcast: Option<BroadcastConfirmMode>) -> FrameInfo {
        FrameInfo::new(source, broadcast, FrameType::Data, PhysAddr::None)
    }

    fn link_status_request_from(source: EndpointAddress) -> FrameInfo {
        FrameInfo::new(source, None, FrameType::LinkStatusRequest, PhysAddr::None)
    }

    /// the primary functions a peer can send, each with its FCV bit set as required
    fn requests(from_master: bool, destination: AnyAddress) -> [Header; 4] {
        [
            header(from_master, Function::PriUnconfirmedUserData, destination),
            header(from_master, Function::PriResetLinkStates, destination),
            confirmed(from_master, true, destination),
            header(from_master, Function::PriRequestLinkStatus, destination),
        ]
    }

    #[test]
    fn outstation_replies_to_link_status_request() {
        let mut layer = outstation(Feature::Disabled);
        let processed = process(
            &mut layer,
            header(true, Function::PriRequestLinkStatus, OUTSTATION.wrap()),
        );
        assert_eq!(processed.info, Some(link_status_request_from(MASTER)));
        assert_eq!(
            processed.reply.as_deref(),
            Some(&LINK_STATUS_FROM_OUTSTATION[..])
        );
    }

    #[test]
    fn outstation_acks_reset_link_states() {
        let mut layer = outstation(Feature::Disabled);
        let processed = process(
            &mut layer,
            header(true, Function::PriResetLinkStates, OUTSTATION.wrap()),
        );
        assert_eq!(processed.info, None);
        assert_eq!(processed.reply.as_deref(), Some(&ACK_FROM_OUTSTATION[..]));
    }

    #[test]
    fn master_replies_to_link_status_request() {
        let mut layer = master();
        let processed = process(
            &mut layer,
            header(false, Function::PriRequestLinkStatus, MASTER.wrap()),
        );
        assert_eq!(processed.info, Some(link_status_request_from(OUTSTATION)));
        assert_eq!(
            processed.reply.as_deref(),
            Some(&LINK_STATUS_FROM_MASTER[..])
        );
    }

    #[test]
    fn master_acks_reset_link_states() {
        let mut layer = master();
        let processed = process(
            &mut layer,
            header(false, Function::PriResetLinkStates, MASTER.wrap()),
        );
        assert_eq!(processed.info, None);
        assert_eq!(processed.reply.as_deref(), Some(&ACK_FROM_MASTER[..]));
    }

    /// confirmed user data is ignored until the link is reset, then acked, with repeats dropped
    fn confirmed_user_data_is_acked_after_reset(
        mut layer: Layer,
        from_master: bool,
        local: EndpointAddress,
        source: EndpointAddress,
        ack: [u8; 10],
    ) {
        let processed = process(&mut layer, confirmed(from_master, true, local.wrap()));
        assert_eq!(processed.info, None);
        assert_eq!(processed.reply, None);
        assert!(matches!(processed.ignored[..], [Ignored::NotReset]));

        let processed = process(
            &mut layer,
            header(from_master, Function::PriResetLinkStates, local.wrap()),
        );
        assert_eq!(processed.reply.as_deref(), Some(&ack[..]));

        let processed = process(&mut layer, confirmed(from_master, true, local.wrap()));
        assert_eq!(processed.info, Some(data_from(source, None)));
        assert_eq!(processed.reply.as_deref(), Some(&ack[..]));
        assert!(processed.ignored.is_empty());

        let processed = process(&mut layer, confirmed(from_master, true, local.wrap()));
        assert_eq!(processed.info, None);
        assert_eq!(processed.reply.as_deref(), Some(&ack[..]));
        assert!(matches!(processed.ignored[..], [Ignored::RepeatedFcb]));

        let processed = process(&mut layer, confirmed(from_master, false, local.wrap()));
        assert_eq!(processed.info, Some(data_from(source, None)));
        assert_eq!(processed.reply.as_deref(), Some(&ack[..]));

        // resetting the layer between sessions requires a new link reset
        layer.reset();
        let processed = process(&mut layer, confirmed(from_master, true, local.wrap()));
        assert!(matches!(processed.ignored[..], [Ignored::NotReset]));
    }

    #[test]
    fn outstation_acks_confirmed_user_data_after_reset() {
        confirmed_user_data_is_acked_after_reset(
            outstation(Feature::Disabled),
            true,
            OUTSTATION,
            MASTER,
            ACK_FROM_OUTSTATION,
        );
    }

    #[test]
    fn master_acks_confirmed_user_data_after_reset() {
        confirmed_user_data_is_acked_after_reset(
            master(),
            false,
            MASTER,
            OUTSTATION,
            ACK_FROM_MASTER,
        );
    }

    #[test]
    fn outstation_passes_up_unconfirmed_user_data_without_reply() {
        let mut layer = outstation(Feature::Disabled);
        let processed = process(
            &mut layer,
            header(true, Function::PriUnconfirmedUserData, OUTSTATION.wrap()),
        );
        assert_eq!(processed.info, Some(data_from(MASTER, None)));
        assert_eq!(processed.reply, None);
    }

    #[test]
    fn master_passes_up_unconfirmed_user_data_without_reply() {
        let mut layer = master();
        let processed = process(
            &mut layer,
            header(false, Function::PriUnconfirmedUserData, MASTER.wrap()),
        );
        assert_eq!(processed.info, Some(data_from(OUTSTATION, None)));
        assert_eq!(processed.reply, None);
    }

    #[test]
    fn both_roles_pass_up_link_status_responses_without_reply() {
        let processed = process(
            &mut master(),
            header(false, Function::SecLinkStatus, MASTER.wrap()),
        );
        assert_eq!(
            processed.info,
            Some(FrameInfo::new(
                OUTSTATION,
                None,
                FrameType::LinkStatusResponse,
                PhysAddr::None
            ))
        );
        assert_eq!(processed.reply, None);

        let processed = process(
            &mut outstation(Feature::Disabled),
            header(true, Function::SecLinkStatus, OUTSTATION.wrap()),
        );
        assert_eq!(
            processed.info,
            Some(FrameInfo::new(
                MASTER,
                None,
                FrameType::LinkStatusResponse,
                PhysAddr::None
            ))
        );
        assert_eq!(processed.reply, None);
    }

    #[test]
    fn master_silently_ignores_frames_from_masters() {
        for destination in [MASTER.wrap(), AnyAddress::SelfAddress] {
            for header in requests(true, destination) {
                assert!(process(&mut master(), header).is_silent());
            }
        }
    }

    #[test]
    fn outstation_silently_ignores_frames_from_outstations() {
        for self_address in [Feature::Disabled, Feature::Enabled] {
            for destination in [OUTSTATION.wrap(), AnyAddress::SelfAddress] {
                for header in requests(false, destination) {
                    assert!(process(&mut outstation(self_address), header).is_silent());
                }
            }
        }
    }

    #[test]
    fn outstation_with_self_address_accepts_user_data_sent_to_self_address() {
        let mut layer = outstation(Feature::Enabled);
        let processed = process(
            &mut layer,
            header(
                true,
                Function::PriUnconfirmedUserData,
                AnyAddress::SelfAddress,
            ),
        );
        assert_eq!(processed.info, Some(data_from(MASTER, None)));
        assert_eq!(processed.reply, None);
        assert!(processed.ignored.is_empty());
    }

    #[test]
    fn outstation_with_self_address_answers_link_status_request_from_its_own_address() {
        let mut layer = outstation(Feature::Enabled);
        let processed = process(
            &mut layer,
            header(
                true,
                Function::PriRequestLinkStatus,
                AnyAddress::SelfAddress,
            ),
        );
        assert_eq!(processed.info, Some(link_status_request_from(MASTER)));
        assert_eq!(
            processed.reply.as_deref(),
            Some(&LINK_STATUS_FROM_OUTSTATION[..])
        );
    }

    #[test]
    fn outstation_with_self_address_acks_confirmed_user_data_sent_to_self_address() {
        let mut layer = outstation(Feature::Enabled);
        let processed = process(
            &mut layer,
            header(true, Function::PriResetLinkStates, AnyAddress::SelfAddress),
        );
        assert_eq!(processed.reply.as_deref(), Some(&ACK_FROM_OUTSTATION[..]));
        let processed = process(&mut layer, confirmed(true, true, AnyAddress::SelfAddress));
        assert_eq!(processed.info, Some(data_from(MASTER, None)));
        assert_eq!(processed.reply.as_deref(), Some(&ACK_FROM_OUTSTATION[..]));
    }

    #[test]
    fn outstation_with_self_address_still_accepts_its_own_address() {
        let mut layer = outstation(Feature::Enabled);
        let processed = process(
            &mut layer,
            header(true, Function::PriUnconfirmedUserData, OUTSTATION.wrap()),
        );
        assert_eq!(processed.info, Some(data_from(MASTER, None)));
    }

    #[test]
    fn outstation_without_self_address_ignores_frames_sent_to_self_address() {
        let mut layer = outstation(Feature::Disabled);
        // reset first, so confirmed user data would otherwise be accepted
        process(
            &mut layer,
            header(true, Function::PriResetLinkStates, OUTSTATION.wrap()),
        );
        for header in requests(true, AnyAddress::SelfAddress) {
            let processed = process(&mut layer, header);
            assert_eq!(processed.info, None);
            assert_eq!(processed.reply, None);
            assert!(matches!(processed.ignored[..], [Ignored::SelfAddress]));
        }
    }

    #[test]
    fn master_ignores_frames_sent_to_self_address() {
        let mut layer = master();
        // reset first, so confirmed user data would otherwise be accepted
        process(
            &mut layer,
            header(false, Function::PriResetLinkStates, MASTER.wrap()),
        );
        for header in requests(false, AnyAddress::SelfAddress) {
            let processed = process(&mut layer, header);
            assert_eq!(processed.info, None);
            assert_eq!(processed.reply, None);
            assert!(matches!(processed.ignored[..], [Ignored::SelfAddress]));
        }
    }

    #[test]
    fn both_roles_ignore_self_address_as_source() {
        let mut from_outstation = header(false, Function::PriUnconfirmedUserData, MASTER.wrap());
        from_outstation.source = AnyAddress::SelfAddress;
        assert!(matches!(
            process(&mut master(), from_outstation).ignored[..],
            [Ignored::UnknownSource(AnyAddress::SelfAddress)]
        ));

        let mut from_master = header(true, Function::PriUnconfirmedUserData, OUTSTATION.wrap());
        from_master.source = AnyAddress::SelfAddress;
        assert!(matches!(
            process(&mut outstation(Feature::Enabled), from_master).ignored[..],
            [Ignored::UnknownSource(AnyAddress::SelfAddress)]
        ));
    }

    #[test]
    fn master_ignores_broadcasts() {
        let mut layer = master();
        process(
            &mut layer,
            header(false, Function::PriResetLinkStates, MASTER.wrap()),
        );
        for mode in BROADCASTS {
            for header in requests(false, AnyAddress::Broadcast(mode)) {
                let processed = process(&mut layer, header);
                assert_eq!(processed.info, None);
                assert_eq!(processed.reply, None);
                assert!(matches!(
                    processed.ignored[..],
                    [Ignored::BroadcastToMaster]
                ));
            }
        }
    }

    #[test]
    fn outstation_accepts_broadcast_user_data_without_reply() {
        for self_address in [Feature::Disabled, Feature::Enabled] {
            let mut layer = outstation(self_address);
            process(
                &mut layer,
                header(true, Function::PriResetLinkStates, OUTSTATION.wrap()),
            );
            let mut fcb = true;
            for mode in BROADCASTS {
                let destination = AnyAddress::Broadcast(mode);
                let processed = process(
                    &mut layer,
                    header(true, Function::PriUnconfirmedUserData, destination),
                );
                assert_eq!(processed.info, Some(data_from(MASTER, Some(mode))));
                assert_eq!(processed.reply, None);

                let processed = process(&mut layer, confirmed(true, fcb, destination));
                assert_eq!(processed.info, Some(data_from(MASTER, Some(mode))));
                assert_eq!(processed.reply, None);
                fcb = !fcb;

                for function in [Function::PriResetLinkStates, Function::PriRequestLinkStatus] {
                    let processed = process(&mut layer, header(true, function, destination));
                    assert_eq!(processed.info, None);
                    assert_eq!(processed.reply, None);
                    assert!(matches!(
                        processed.ignored[..],
                        [Ignored::BroadcastFunction(f)] if f == function
                    ));
                }
            }
        }
    }
}
