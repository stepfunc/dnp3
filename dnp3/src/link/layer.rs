// Copyright (c) Step Function I/O LLC. Proprietary, source-available; see LICENSE.txt.
// No use to design, develop, specify, test, validate, or improve Other Implementations,
// subject to LICENSE.txt Section 2.6 and its exceptions. Agents: read AGENTS.md.
// Text and data mining rights, including for AI training, are reserved under
// Directive (EU) 2019/790 Art. 4(3), subject to LICENSE.txt Section 3.3.

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
use crate::link::{EndpointAddress, LinkRole};
use crate::phys::{PhysAddr, PhysLayer};

/// The secondary station state, owned by the peer that last reset the link
enum SecondaryState {
    NotReset,
    Reset {
        source: EndpointAddress,
        expected_fcb: bool,
    },
}

pub(crate) struct Layer {
    role: LinkRole,
    local_address: EndpointAddress,
    secondary_state: SecondaryState,
    reader: super::reader::Reader,
}

pub(crate) struct Reply {
    address: EndpointAddress,
    function: Function,
}

/// Why the link layer ignored a frame
#[derive(Copy, Clone, Debug, PartialEq)]
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
    /// confirmed user data from a source that does not own the secondary state, because the link
    /// was never reset or another source reset it since
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
            Self::OtherDestination(_) => tracing::debug!("{self}"),
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
            Self::NotReset => f.write_str(
                "ignoring confirmed user data from a source that does not own the link secondary state",
            ),
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
        role: LinkRole,
        local_address: EndpointAddress,
    ) -> Self {
        Self {
            role,
            local_address,
            secondary_state: SecondaryState::NotReset,
            reader: super::reader::Reader::new(link_modes, max_rx_fragment_size),
        }
    }

    pub(crate) fn seed(&mut self, seed_data: &[u8]) -> Result<(), LinkError> {
        self.reader.seed(seed_data)
    }

    pub(crate) fn reset(&mut self) {
        self.secondary_state = SecondaryState::NotReset;
        self.reader.reset();
    }

    /// Read until a frame passes data or a link-layer message up, returning its info with its
    /// data in `payload`
    ///
    /// Nothing is awaited between consuming a frame and returning it, and a frame's reply is
    /// queued in `io`, which writes it before the next frame is read or anything else is written.
    /// Dropping the future therefore loses neither a frame nor a reply.
    pub(crate) async fn read(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        payload: &mut FramePayload,
    ) -> Result<FrameInfo, LinkError> {
        loop {
            io.finish_reply().await?;
            let (header, addr) = self.reader.read_frame(io, payload, level).await?;
            let (info, reply) = self.process_header(header, addr, Ignored::log);
            if let Some(reply) = reply {
                let header = self.get_header(reply);
                if level.link.enabled() {
                    tracing::info!("LINK TX - {}", LinkDisplay::new(header, &[], level.link));
                }
                io.queue_reply(Self::format_reply(header), addr, level.physical)?;
            }
            if let Some(info) = info {
                return Ok(info);
            }
        }
    }

    pub(crate) fn get_header(&self, reply: Reply) -> Header {
        Header::new(
            ControlField::new(self.role.dir_bit(), reply.function),
            reply.address.wrap(),
            self.local_address.wrap(),
        )
    }

    pub(crate) fn format_reply(header: Header) -> [u8; super::constant::LINK_HEADER_LENGTH] {
        header_bytes(header, super::constant::MIN_HEADER_LENGTH_VALUE)
    }
}

mod header_processing {
    use super::*;

    impl Layer {
        /// Process a frame's header, passing the reason to `ignore` whenever the frame is ignored
        pub(crate) fn process_header<F: FnMut(Ignored)>(
            &mut self,
            header: Header,
            addr: PhysAddr,
            mut ignore: F,
        ) -> (Option<FrameInfo>, Option<Reply>) {
            // ignore frames sent from the same endpoint type
            if header.control.master == self.role.dir_bit() {
                // we don't log this
                return (None, None);
            }

            let (source, broadcast) =
                match validate_addresses(self.role, self.local_address, header) {
                    Ok(addresses) => addresses,
                    Err(reason) => {
                        ignore(reason);
                        return (None, None);
                    }
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

                    self.secondary_state = SecondaryState::Reset {
                        source,
                        expected_fcb: true,
                    };
                    (None, Some(Reply::new(source, Function::SecAck)))
                }
                Function::PriConfirmedUserData => process_confirmed_data(
                    &mut self.secondary_state,
                    header.control,
                    FrameInfo::new(source, broadcast, FrameType::Data, addr),
                    &mut ignore,
                ),
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

    fn validate_addresses(
        role: LinkRole,
        local_address: EndpointAddress,
        header: Header,
    ) -> Result<(EndpointAddress, Option<BroadcastConfirmMode>), Ignored> {
        let source = match header.source {
            AnyAddress::Endpoint(source) => source,
            AnyAddress::Reserved(_) | AnyAddress::Broadcast(_) | AnyAddress::SelfAddress => {
                return Err(Ignored::UnknownSource(header.source));
            }
        };

        let broadcast = match (role, header.destination) {
            (_, AnyAddress::Endpoint(destination)) if destination == local_address => None,
            (_, AnyAddress::Endpoint(destination)) => {
                return Err(Ignored::OtherDestination(destination));
            }
            (LinkRole::Outstation { self_address: true }, AnyAddress::SelfAddress) => None,
            (_, AnyAddress::SelfAddress) => return Err(Ignored::SelfAddress),
            (_, AnyAddress::Reserved(destination)) => {
                return Err(Ignored::ReservedDestination(destination));
            }
            (LinkRole::Master, AnyAddress::Broadcast(_)) => return Err(Ignored::BroadcastToMaster),
            (LinkRole::Outstation { .. }, AnyAddress::Broadcast(mode)) => Some(mode),
        };

        Ok((source, broadcast))
    }

    fn process_confirmed_data<F: FnMut(Ignored)>(
        state: &mut SecondaryState,
        control: ControlField,
        info: FrameInfo,
        ignore: &mut F,
    ) -> (Option<FrameInfo>, Option<Reply>) {
        if !control.fcv {
            ignore(Ignored::ConfirmedWithoutFcv);
            return (None, None);
        }

        let expected = match *state {
            SecondaryState::Reset {
                source,
                expected_fcb,
            } if source == info.source => expected_fcb,
            SecondaryState::NotReset | SecondaryState::Reset { .. } => {
                ignore(Ignored::NotReset);
                return (None, None);
            }
        };

        let reply = if info.broadcast.is_none() {
            Some(Reply::new(info.source, Function::SecAck))
        } else {
            None
        };

        if control.fcb != expected {
            ignore(Ignored::RepeatedFcb);
            return (None, reply);
        }

        *state = SecondaryState::Reset {
            source: info.source,
            expected_fcb: !expected,
        };
        (Some(info), reply)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::util::test_support::{ACK_FROM_OUTSTATION, MASTER_REPLY, OUTSTATION_REPLY};

    const MASTER: EndpointAddress = EndpointAddress::raw(1);
    const OUTSTATION: EndpointAddress = EndpointAddress::raw(1024);

    const ACK_FROM_MASTER: [u8; 10] = [0x05, 0x64, 0x05, 0x80, 0x00, 0x04, 0x01, 0x00, 0x6D, 0xC7];

    const BROADCASTS: [BroadcastConfirmMode; 3] = [
        BroadcastConfirmMode::Optional,
        BroadcastConfirmMode::Mandatory,
        BroadcastConfirmMode::NotRequired,
    ];

    fn master() -> Layer {
        Layer::new(LinkModes::test(), 2048, LinkRole::Master, MASTER)
    }

    fn outstation(self_address: bool) -> Layer {
        Layer::new(
            LinkModes::test(),
            2048,
            LinkRole::Outstation { self_address },
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

    struct Processed {
        info: Option<FrameInfo>,
        reply: Option<Vec<u8>>,
    }

    fn process(layer: &mut Layer, header: Header) -> Processed {
        let (info, reply) = layer.process_header(header, PhysAddr::None, |_| {});
        let reply = reply.map(|reply| {
            let header = layer.get_header(reply);
            Layer::format_reply(header).to_vec()
        });
        Processed { info, reply }
    }

    fn data_from(source: EndpointAddress, broadcast: Option<BroadcastConfirmMode>) -> FrameInfo {
        FrameInfo::new(source, broadcast, FrameType::Data, PhysAddr::None)
    }

    fn link_status_request_from(source: EndpointAddress) -> FrameInfo {
        FrameInfo::new(source, None, FrameType::LinkStatusRequest, PhysAddr::None)
    }

    #[test]
    fn outstation_replies_to_link_status_request() {
        let mut layer = outstation(false);
        let processed = process(
            &mut layer,
            header(true, Function::PriRequestLinkStatus, OUTSTATION.wrap()),
        );
        assert_eq!(processed.info, Some(link_status_request_from(MASTER)));
        assert_eq!(processed.reply.as_deref(), Some(&OUTSTATION_REPLY[..]));
    }

    #[test]
    fn outstation_acks_reset_link_states() {
        let mut layer = outstation(false);
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
        assert_eq!(processed.reply.as_deref(), Some(&MASTER_REPLY[..]));
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

    /// the lowest and highest reserved addresses
    const RESERVED: [u16; 2] = [0xFFF0, 0xFFFB];

    /// the functions the link layer never acts on: test link states, the secondary functions
    /// other than link status, and unknown primary and secondary codes
    const IGNORED_FUNCTIONS: [Function; 6] = [
        Function::PriTestLinkStates,
        Function::SecAck,
        Function::SecNack,
        Function::SecNotSupported,
        Function::Unknown(0x41),
        Function::Unknown(0x0E),
    ];

    /// Property: for a master and an outstation with and without self-address support, frame by
    /// frame through up to 64 frames and session ends, what `process_header` delivers, the reply
    /// it asks for and the reason it ignores a frame follow a model in which the peer whose reset
    /// was last accepted owns the secondary state and its FCB, until the session ends. Frames that
    /// fail any check leave the state unchanged, and a reply's header carries the layer's DIR bit
    /// and its own address.
    ///
    /// The model is written from the rules and is independent of the layer. Where several
    /// reasons to ignore a frame apply, it accepts any one of them, so which one the layer
    /// reports, which is only logged, isn't pinned. The expected reply header is built with
    /// `ControlField::new`; the four reply examples above pin the reply bytes.
    ///
    /// The generator draws three peers, favoring the first so it often sends again after its own
    /// reset; functions weighted toward reset and confirmed user data, plus every function the
    /// layer never acts on; an FCV bit that is wrong for the function now and then and any FCB;
    /// destinations that are mostly the layer's own address, and otherwise the self address, a
    /// broadcast address, the reserved bounds or another endpoint; sources that are mostly a peer
    /// and otherwise the self, broadcast or reserved addresses; a frame from the same type of
    /// endpoint now and then; and a UDP source address or none. It is built to reach ownership
    /// passing between peers, repeated FCBs (acknowledged, not delivered), confirmed broadcasts
    /// and every ignore reason. The fixed op lists below run particular histories through the
    /// same model. Not exercised: `Layer::read`, which reads frames and queues the replies through
    /// the physical layer (the transport reader and `phys/mod.rs` tests cover it).
    mod properties {
        use super::*;
        use proptest::prelude::*;
        use std::net::{Ipv4Addr, SocketAddr};

        /// the peers that send frames, any of which can own the secondary state
        const PEERS: [EndpointAddress; 3] = [
            EndpointAddress::raw(10),
            EndpointAddress::raw(11),
            EndpointAddress::raw(12),
        ];

        /// an endpoint address that isn't the layer's own
        const OTHER: EndpointAddress = EndpointAddress::raw(2);

        const ROLES: [LinkRole; 3] = [
            LinkRole::Master,
            LinkRole::Outstation {
                self_address: false,
            },
            LinkRole::Outstation { self_address: true },
        ];

        fn local(role: LinkRole) -> EndpointAddress {
            match role {
                LinkRole::Master => MASTER,
                LinkRole::Outstation { .. } => OUTSTATION,
            }
        }

        fn is_master(role: LinkRole) -> bool {
            matches!(role, LinkRole::Master)
        }

        /// whether a frame with `func` must have its FCV bit set
        fn needs_fcv(func: Function) -> bool {
            matches!(func, Function::PriConfirmedUserData)
        }

        #[derive(Copy, Clone, Debug)]
        enum Destination {
            /// the layer's own address
            Local,
            Other(AnyAddress),
        }

        #[derive(Copy, Clone, Debug)]
        struct Frame {
            /// sent by the same type of endpoint as the layer
            same_type: bool,
            func: Function,
            fcb: bool,
            fcv: bool,
            destination: Destination,
            source: AnyAddress,
            addr: PhysAddr,
        }

        impl Frame {
            /// `func` from `peer` to the layer's own address, with FCB clear and the FCV bit the
            /// function needs
            fn new(func: Function, peer: EndpointAddress) -> Self {
                Self {
                    same_type: false,
                    func,
                    fcb: false,
                    fcv: needs_fcv(func),
                    destination: Destination::Local,
                    source: peer.wrap(),
                    addr: PhysAddr::None,
                }
            }

            fn to(self, destination: AnyAddress) -> Self {
                Self {
                    destination: Destination::Other(destination),
                    ..self
                }
            }

            fn from(self, source: AnyAddress) -> Self {
                Self { source, ..self }
            }

            fn fcb(self, fcb: bool) -> Self {
                Self { fcb, ..self }
            }

            fn fcv(self, fcv: bool) -> Self {
                Self { fcv, ..self }
            }

            fn same_type(self) -> Self {
                Self {
                    same_type: true,
                    ..self
                }
            }

            fn header(&self, role: LinkRole) -> Header {
                let control = ControlField {
                    func: self.func,
                    master: is_master(role) == self.same_type,
                    fcb: self.fcb,
                    fcv: self.fcv,
                };
                let destination = match self.destination {
                    Destination::Local => local(role).wrap(),
                    Destination::Other(x) => x,
                };
                Header::new(control, destination, self.source)
            }
        }

        #[derive(Copy, Clone, Debug)]
        enum Op {
            Frame(Frame),
            /// the session ended, so the layer is reset before the next one
            EndSession,
        }

        /// The link secondary state, with the policy that one peer owns it: the peer whose reset was
        /// last accepted, until the session ends
        #[derive(Copy, Clone, Debug, PartialEq)]
        enum Model {
            NotReset,
            Reset {
                owner: EndpointAddress,
                expected_fcb: bool,
            },
        }

        /// What the layer did with a frame
        #[derive(Clone, Debug, PartialEq)]
        struct Outcome {
            info: Option<FrameInfo>,
            reply: Option<(EndpointAddress, Function)>,
            ignored: Vec<Ignored>,
        }

        /// What the layer must do with a frame
        #[derive(Clone, Debug)]
        struct Expected {
            info: Option<FrameInfo>,
            reply: Option<(EndpointAddress, Function)>,
            /// every reason that applies to a frame the layer must ignore; it reports one of them
            reasons: Vec<Ignored>,
        }

        impl Expected {
            fn silent() -> Self {
                Self {
                    info: None,
                    reply: None,
                    reasons: Vec::new(),
                }
            }

            fn ignored(reasons: Vec<Ignored>) -> Self {
                Self {
                    info: None,
                    reply: None,
                    reasons,
                }
            }

            fn accepts(&self, actual: &Outcome) -> bool {
                let ignored = match actual.ignored[..] {
                    [] => self.reasons.is_empty(),
                    [reason] => self.reasons.contains(&reason),
                    _ => false,
                };
                ignored && actual.info == self.info && actual.reply == self.reply
            }
        }

        impl Model {
            fn step(&mut self, role: LinkRole, header: Header, addr: PhysAddr) -> Expected {
                let control = header.control;
                // a frame from an endpoint of the same type is dropped without a reason
                if control.master == is_master(role) {
                    return Expected::silent();
                }

                let mut reasons = Vec::new();
                let source = match header.source {
                    AnyAddress::Endpoint(x) => Some(x),
                    other => {
                        reasons.push(Ignored::UnknownSource(other));
                        None
                    }
                };

                let is_user_data = matches!(
                    control.func,
                    Function::PriUnconfirmedUserData | Function::PriConfirmedUserData
                );
                let mut broadcast = None;
                match (header.destination, role) {
                    (AnyAddress::Endpoint(x), _) if x == local(role) => {}
                    (AnyAddress::Endpoint(x), _) => reasons.push(Ignored::OtherDestination(x)),
                    (AnyAddress::SelfAddress, LinkRole::Outstation { self_address: true }) => {}
                    (AnyAddress::SelfAddress, _) => reasons.push(Ignored::SelfAddress),
                    (AnyAddress::Reserved(x), _) => reasons.push(Ignored::ReservedDestination(x)),
                    (AnyAddress::Broadcast(_), LinkRole::Master) => {
                        reasons.push(Ignored::BroadcastToMaster)
                    }
                    (AnyAddress::Broadcast(mode), LinkRole::Outstation { .. }) => {
                        broadcast = Some(mode);
                        if !is_user_data {
                            reasons.push(Ignored::BroadcastFunction(control.func));
                        }
                    }
                }

                match (control.func, control.fcv) {
                    (Function::PriUnconfirmedUserData, true) => {
                        reasons.push(Ignored::UnconfirmedWithFcv)
                    }
                    (Function::PriResetLinkStates, true) => reasons.push(Ignored::ResetWithFcv),
                    (Function::PriConfirmedUserData, false) => {
                        reasons.push(Ignored::ConfirmedWithoutFcv)
                    }
                    (Function::PriRequestLinkStatus, true) => {
                        reasons.push(Ignored::LinkStatusWithFcv)
                    }
                    (
                        Function::PriUnconfirmedUserData
                        | Function::PriResetLinkStates
                        | Function::PriConfirmedUserData
                        | Function::PriRequestLinkStatus
                        | Function::SecLinkStatus,
                        _,
                    ) => {}
                    (
                        function @ (Function::PriTestLinkStates
                        | Function::SecAck
                        | Function::SecNack
                        | Function::SecNotSupported
                        | Function::Unknown(_)),
                        _,
                    ) => reasons.push(Ignored::Function(function)),
                }

                let Some(source) = source.filter(|_| reasons.is_empty()) else {
                    return Expected::ignored(reasons);
                };
                let info = |frame_type| Some(FrameInfo::new(source, broadcast, frame_type, addr));

                match control.func {
                    Function::PriUnconfirmedUserData => Expected {
                        info: info(FrameType::Data),
                        reply: None,
                        reasons,
                    },
                    Function::PriResetLinkStates => {
                        *self = Model::Reset {
                            owner: source,
                            expected_fcb: true,
                        };
                        Expected {
                            info: None,
                            reply: Some((source, Function::SecAck)),
                            reasons,
                        }
                    }
                    Function::PriConfirmedUserData => match *self {
                        Model::Reset {
                            owner,
                            expected_fcb,
                        } if owner == source => {
                            let reply = match broadcast {
                                None => Some((source, Function::SecAck)),
                                Some(_) => None,
                            };
                            if control.fcb == expected_fcb {
                                *self = Model::Reset {
                                    owner,
                                    expected_fcb: !expected_fcb,
                                };
                                Expected {
                                    info: info(FrameType::Data),
                                    reply,
                                    reasons,
                                }
                            } else {
                                Expected {
                                    info: None,
                                    reply,
                                    reasons: vec![Ignored::RepeatedFcb],
                                }
                            }
                        }
                        Model::NotReset | Model::Reset { .. } => {
                            Expected::ignored(vec![Ignored::NotReset])
                        }
                    },
                    Function::PriRequestLinkStatus => Expected {
                        info: info(FrameType::LinkStatusRequest),
                        reply: Some((source, Function::SecLinkStatus)),
                        reasons,
                    },
                    Function::SecLinkStatus => Expected {
                        info: info(FrameType::LinkStatusResponse),
                        reply: None,
                        reasons,
                    },
                    Function::PriTestLinkStates
                    | Function::SecAck
                    | Function::SecNack
                    | Function::SecNotSupported
                    | Function::Unknown(_) => Expected::ignored(reasons),
                }
            }
        }

        /// process a frame, checking that a reply's header carries the layer's DIR bit and address
        fn outcome(
            layer: &mut Layer,
            role: LinkRole,
            header: Header,
            addr: PhysAddr,
        ) -> Result<Outcome, TestCaseError> {
            let mut ignored = Vec::new();
            let (info, reply) = layer.process_header(header, addr, |x| ignored.push(x));
            let reply = reply.map(|reply| (reply.address, reply.function, layer.get_header(reply)));
            if let Some((destination, function, header)) = reply {
                prop_assert_eq!(
                    header,
                    Header::new(
                        ControlField::new(is_master(role), function),
                        destination.wrap(),
                        local(role).wrap(),
                    )
                );
            }
            Ok(Outcome {
                info,
                reply: reply.map(|(destination, function, _)| (destination, function)),
                ignored,
            })
        }

        fn role() -> impl Strategy<Value = LinkRole> {
            prop::sample::select(ROLES.to_vec())
        }

        /// a peer, favoring the first so it often sends again after its own reset
        fn peer() -> impl Strategy<Value = EndpointAddress> {
            let [a, b, c] = PEERS;
            prop_oneof![
                3 => Just(a),
                1 => Just(b),
                1 => Just(c),
            ]
        }

        fn broadcast() -> impl Strategy<Value = AnyAddress> {
            prop::sample::select(BROADCASTS.to_vec()).prop_map(AnyAddress::Broadcast)
        }

        fn reserved() -> impl Strategy<Value = AnyAddress> {
            prop::sample::select(RESERVED.to_vec()).prop_map(AnyAddress::Reserved)
        }

        fn source() -> impl Strategy<Value = AnyAddress> {
            prop_oneof![
                24 => peer().prop_map(|x| x.wrap()),
                1 => Just(AnyAddress::SelfAddress),
                1 => broadcast(),
                1 => reserved(),
            ]
        }

        fn destination() -> impl Strategy<Value = Destination> {
            prop_oneof![
                16 => Just(Destination::Local),
                2 => Just(Destination::Other(AnyAddress::SelfAddress)),
                3 => broadcast().prop_map(Destination::Other),
                1 => reserved().prop_map(Destination::Other),
                1 => prop_oneof![Just(OTHER), peer()].prop_map(|x| Destination::Other(x.wrap())),
            ]
        }

        fn function() -> impl Strategy<Value = Function> {
            prop_oneof![
                6 => Just(Function::PriResetLinkStates),
                12 => Just(Function::PriConfirmedUserData),
                2 => Just(Function::PriUnconfirmedUserData),
                2 => Just(Function::PriRequestLinkStatus),
                1 => Just(Function::SecLinkStatus),
                1 => prop::sample::select(IGNORED_FUNCTIONS.to_vec()),
            ]
        }

        fn phys_addr() -> impl Strategy<Value = PhysAddr> {
            prop_oneof![
                Just(PhysAddr::None),
                Just(PhysAddr::Udp(SocketAddr::from((
                    Ipv4Addr::LOCALHOST,
                    20000
                )))),
            ]
        }

        /// a function and an FCV bit that is mostly, but not always, the one the function needs
        fn function_and_fcv() -> impl Strategy<Value = (Function, bool)> {
            (function(), prop::bool::weighted(0.15))
                .prop_map(|(func, wrong)| (func, needs_fcv(func) != wrong))
        }

        /// a frame, mostly from the other type of endpoint, with arbitrary FCB
        fn op() -> impl Strategy<Value = Op> {
            let frame = (
                prop::bool::weighted(0.1),
                function_and_fcv(),
                any::<bool>(),
                destination(),
                source(),
                phys_addr(),
            )
                .prop_map(
                    |(same_type, (func, fcv), fcb, destination, source, addr)| Frame {
                        same_type,
                        func,
                        fcb,
                        fcv,
                        destination,
                        source,
                        addr,
                    },
                )
                .prop_map(Op::Frame);
            prop_oneof![
                30 => frame,
                1 => Just(Op::EndSession),
            ]
        }

        /// Run `ops` through a layer with `role`, checking each frame's outcome against the model
        fn check(role: LinkRole, ops: impl IntoIterator<Item = Op>) -> Result<(), TestCaseError> {
            let mut layer = Layer::new(LinkModes::test(), 2048, role, local(role));
            let mut model = Model::NotReset;
            for (step, op) in ops.into_iter().enumerate() {
                match op {
                    Op::Frame(frame) => {
                        let (header, addr) = (frame.header(role), frame.addr);
                        let expected = model.step(role, header, addr);
                        let actual = outcome(&mut layer, role, header, addr)?;
                        prop_assert!(
                            expected.accepts(&actual),
                            "step {}: {:?}\nexpected {:?}\nactual {:?}",
                            step,
                            header,
                            expected,
                            actual
                        );
                    }
                    Op::EndSession => {
                        layer.reset();
                        model = Model::NotReset;
                    }
                }
            }
            Ok(())
        }

        proptest! {
            #![proptest_config(crate::util::test_support::proptest_config(128))]

            /// Delivery, replies and ignore reasons follow a model in which the peer whose reset
            /// was last accepted owns the secondary state and its FCB, and frames that fail any
            /// check leave the state unchanged
            #[test]
            fn frames_follow_the_one_owner_secondary_state_model(
                role in role(),
                ops in prop::collection::vec(op(), 1..=64),
            ) {
                check(role, ops)?;
            }
        }

        /// Run `ops` through a layer of each role, checking each frame's outcome against the model
        fn check_each_role(ops: &[Op]) {
            for role in ROLES {
                if let Err(err) = check(role, ops.iter().copied()) {
                    panic!("{role:?}: {err}");
                }
            }
        }

        /// the primary functions a peer sends
        const REQUESTS: [Function; 4] = [
            Function::PriUnconfirmedUserData,
            Function::PriResetLinkStates,
            Function::PriConfirmedUserData,
            Function::PriRequestLinkStatus,
        ];

        fn reset(peer: EndpointAddress) -> Op {
            Op::Frame(Frame::new(Function::PriResetLinkStates, peer))
        }

        fn confirmed_data(peer: EndpointAddress, fcb: bool) -> Op {
            Op::Frame(Frame::new(Function::PriConfirmedUserData, peer).fcb(fcb))
        }

        /// `frames` after the first peer resets the link and sends one confirmed frame, then the
        /// next confirmed frame it sends, so a frame that changed the secondary state shows in
        /// that last one
        fn between_owner_frames(frames: impl IntoIterator<Item = Frame>) -> Vec<Op> {
            let [a, _, _] = PEERS;
            let mut ops = vec![reset(a), confirmed_data(a, true)];
            ops.extend(frames.into_iter().map(Op::Frame));
            ops.push(confirmed_data(a, false));
            ops
        }

        /// Confirmed user data is ignored until the link is reset, then delivered and acked once
        /// per FCB toggle, and needs a new reset after the session ends
        #[test]
        fn confirmed_user_data_is_delivered_after_reset() {
            let [a, _, _] = PEERS;
            check_each_role(&[
                confirmed_data(a, true),
                reset(a),
                confirmed_data(a, true),
                confirmed_data(a, true),
                confirmed_data(a, false),
                Op::EndSession,
                confirmed_data(a, true),
            ]);
        }

        /// Only the peer that last reset the link may send confirmed user data, and another
        /// peer's frames leave the owner's FCB unchanged
        #[test]
        fn secondary_state_is_owned_by_the_last_peer_to_reset_it() {
            let [a, b, _] = PEERS;
            check_each_role(&[
                reset(a),
                confirmed_data(a, true),
                confirmed_data(b, false),
                confirmed_data(a, false),
                confirmed_data(b, true),
                confirmed_data(a, true),
                confirmed_data(a, true),
                reset(b),
                confirmed_data(a, true),
                confirmed_data(b, true),
                confirmed_data(b, true),
            ]);
        }

        /// Unconfirmed user data, link status requests and link status responses are passed up
        #[test]
        fn user_data_and_link_status_are_passed_up() {
            let [a, _, _] = PEERS;
            check_each_role(
                &[
                    Function::PriUnconfirmedUserData,
                    Function::PriRequestLinkStatus,
                    Function::SecLinkStatus,
                ]
                .map(|func| Op::Frame(Frame::new(func, a))),
            );
        }

        /// Frames from the same type of endpoint are dropped without a reason
        #[test]
        fn frames_from_the_same_type_of_endpoint_are_dropped() {
            let [a, _, _] = PEERS;
            let frames = REQUESTS.map(|func| Frame::new(func, a).same_type());
            check_each_role(&between_owner_frames(
                frames
                    .into_iter()
                    .chain(frames.map(|x| x.to(AnyAddress::SelfAddress))),
            ));
        }

        /// Only an outstation with the feature enabled accepts frames sent to the self address,
        /// and it answers from its own address
        #[test]
        fn frames_sent_to_the_self_address() {
            let [a, _, _] = PEERS;
            check_each_role(&between_owner_frames(
                [
                    Frame::new(Function::PriUnconfirmedUserData, a),
                    Frame::new(Function::PriConfirmedUserData, a),
                    Frame::new(Function::PriRequestLinkStatus, a),
                    Frame::new(Function::PriResetLinkStates, a),
                    Frame::new(Function::PriConfirmedUserData, a).fcb(true),
                ]
                .map(|x| x.to(AnyAddress::SelfAddress)),
            ));
        }

        #[test]
        fn frames_from_the_self_address_are_ignored() {
            let [a, _, _] = PEERS;
            check_each_role(&between_owner_frames(
                REQUESTS.map(|func| Frame::new(func, a).from(AnyAddress::SelfAddress)),
            ));
        }

        /// A master ignores broadcasts; an outstation passes up broadcast user data with its mode
        /// and without a reply, even when confirmed, and ignores other broadcast functions
        #[test]
        fn broadcasts() {
            let [a, _, _] = PEERS;
            let mut fcb = false;
            let frames = BROADCASTS.map(|mode| {
                let frames = REQUESTS
                    .map(|func| Frame::new(func, a).fcb(fcb).to(AnyAddress::Broadcast(mode)));
                fcb = !fcb;
                frames
            });
            check_each_role(&between_owner_frames(frames.into_iter().flatten()));
        }

        #[test]
        fn frames_with_the_wrong_fcv_bit_are_ignored() {
            let [a, _, _] = PEERS;
            check_each_role(&between_owner_frames(
                REQUESTS.map(|func| Frame::new(func, a).fcv(!needs_fcv(func))),
            ));
        }

        #[test]
        fn frames_to_reserved_addresses_are_ignored() {
            let [a, _, _] = PEERS;
            for address in RESERVED {
                assert_eq!(AnyAddress::from(address), AnyAddress::Reserved(address));
            }
            check_each_role(&between_owner_frames(RESERVED.into_iter().flat_map(
                |address| REQUESTS.map(|func| Frame::new(func, a).to(AnyAddress::from(address))),
            )));
        }

        /// Frames with a function the link layer doesn't act on are ignored whatever their FCB and
        /// FCV bits
        #[test]
        fn frames_with_functions_the_layer_does_not_act_on_are_ignored() {
            let [a, _, _] = PEERS;
            check_each_role(&between_owner_frames(
                IGNORED_FUNCTIONS.into_iter().flat_map(|func| {
                    [(false, false), (false, true), (true, true)]
                        .map(|(fcb, fcv)| Frame::new(func, a).fcb(fcb).fcv(fcv))
                }),
            ));
        }

        fn confirmed_from(role: LinkRole, source: EndpointAddress, fcb: bool) -> Header {
            Frame::new(Function::PriConfirmedUserData, source)
                .fcb(fcb)
                .header(role)
        }

        fn reset_from(role: LinkRole, source: EndpointAddress) -> Header {
            Frame::new(Function::PriResetLinkStates, source).header(role)
        }

        /// The model rejects data from a peer that no longer owns the link and a redelivered
        /// repeat, and leaves the state alone for a frame that fails a check
        #[test]
        fn model_rejects_a_second_owner_and_a_redelivered_repeat() {
            let role = LinkRole::Outstation {
                self_address: false,
            };
            let [a, b, _] = PEERS;
            let mut model = Model::NotReset;
            for source in [a, b] {
                let expected = model.step(role, reset_from(role, source), PhysAddr::None);
                assert_eq!(expected.reply, Some((source, Function::SecAck)));
            }

            // `a` reset the link earlier, but `b` owns it now
            let expected = model.step(role, confirmed_from(role, a, true), PhysAddr::None);
            let delivered = Outcome {
                info: Some(data_from(a, None)),
                reply: Some((a, Function::SecAck)),
                ignored: Vec::new(),
            };
            assert!(!expected.accepts(&delivered));
            assert!(expected.accepts(&Outcome {
                info: None,
                reply: None,
                ignored: vec![Ignored::NotReset],
            }));

            // the owner's repeat is acked but not delivered again
            let expected = model.step(role, confirmed_from(role, b, true), PhysAddr::None);
            assert!(expected.accepts(&Outcome {
                info: Some(data_from(b, None)),
                reply: Some((b, Function::SecAck)),
                ignored: Vec::new(),
            }));
            let expected = model.step(role, confirmed_from(role, b, true), PhysAddr::None);
            assert!(!expected.accepts(&Outcome {
                info: Some(data_from(b, None)),
                reply: Some((b, Function::SecAck)),
                ignored: Vec::new(),
            }));
            assert!(expected.accepts(&Outcome {
                info: None,
                reply: Some((b, Function::SecAck)),
                ignored: vec![Ignored::RepeatedFcb],
            }));

            // a frame failing two checks may report either
            let mut header = reset_from(role, a);
            header.control.fcv = true;
            header.destination = OTHER.wrap();
            let expected = model.step(role, header, PhysAddr::None);
            for reason in [Ignored::ResetWithFcv, Ignored::OtherDestination(OTHER)] {
                assert!(expected.accepts(&Outcome {
                    info: None,
                    reply: None,
                    ignored: vec![reason],
                }));
            }
            assert_eq!(
                model,
                Model::Reset {
                    owner: b,
                    expected_fcb: false
                }
            );
        }
    }
}
