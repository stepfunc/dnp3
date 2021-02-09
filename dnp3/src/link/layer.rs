use crate::app::EndpointType;
use crate::decode::DecodeLevel;
use crate::link::display::LinkDisplay;
use crate::link::error::LinkError;
use crate::link::format::format_header_fixed_size;
use crate::link::function::Function;
use crate::link::header::{
    AnyAddress, BroadcastConfirmMode, ControlField, FrameInfo, FrameType, Header,
};
use crate::link::parser::FramePayload;
use crate::link::{EndpointAddress, LinkErrorMode};
use crate::outstation::Feature;
use crate::util::phys::PhysLayer;

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

struct Reply {
    address: EndpointAddress,
    function: Function,
}

impl Reply {
    fn new(address: EndpointAddress, function: Function) -> Self {
        Self { address, function }
    }
}

impl Layer {
    pub(crate) fn new(
        error_mode: LinkErrorMode,
        endpoint_type: EndpointType,
        self_address: Feature,
        local_address: EndpointAddress,
    ) -> Self {
        Self {
            endpoint_type,
            self_address,
            local_address,
            secondary_state: SecondaryState::NotReset,
            reader: super::reader::Reader::new(error_mode),
            tx_buffer: [0; super::constant::LINK_HEADER_LENGTH],
        }
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

    fn get_header(&self, reply: Reply) -> Header {
        Header::new(
            ControlField::new(self.endpoint_type.dir_bit(), reply.function),
            reply.address.wrap(),
            self.local_address.wrap(),
        )
    }

    fn format_reply(&mut self, header: Header) -> &[u8] {
        format_header_fixed_size(header, &mut self.tx_buffer);
        &self.tx_buffer
    }

    async fn read_one(
        &mut self,
        io: &mut PhysLayer,
        level: DecodeLevel,
        payload: &mut FramePayload,
    ) -> Result<Option<FrameInfo>, LinkError> {
        let header = self.reader.read(io, payload, level).await?;
        let (info, reply) = self.process_header(&header);
        if let Some(reply) = reply {
            let header = self.get_header(reply);
            if level.link.enabled() {
                tracing::info!("LINK TX - {}", LinkDisplay::new(header, &[], level.link));
            }
            io.write(self.format_reply(header), level.physical).await?
        }
        Ok(info)
    }

    fn process_header(&mut self, header: &Header) -> (Option<FrameInfo>, Option<Reply>) {
        // ignore frames sent from the same endpoint type
        if header.control.master == self.endpoint_type.dir_bit() {
            //  we don't log this
            return (None, None);
        }

        // validate the source address
        let source: EndpointAddress = match header.source {
            AnyAddress::Endpoint(x) => x,
            _ => {
                tracing::warn!(
                    "ignoring frame from disallowed source address: {}",
                    header.source
                );
                return (None, None);
            }
        };

        // validate the destination address
        let broadcast: Option<BroadcastConfirmMode> = match header.destination {
            AnyAddress::Endpoint(x) => {
                if x == self.local_address {
                    None
                } else {
                    tracing::warn!("ignoring frame sent to address: {}", x);
                    return (None, None);
                }
            }
            AnyAddress::SelfAddress => {
                if self.self_address.is_enabled() {
                    // just pretend like it was sent to us
                    None
                } else {
                    tracing::warn!("ignoring frame sent to self address");
                    return (None, None);
                }
            }
            AnyAddress::Reserved(x) => {
                tracing::warn!("ignoring frame sent to reserved address: {}", x);
                return (None, None);
            }
            AnyAddress::Broadcast(mode) => match self.endpoint_type {
                EndpointType::Master => {
                    tracing::warn!("ignoring broadcast frame sent to master");
                    return (None, None);
                }
                EndpointType::Outstation => Some(mode),
            },
        };

        // broadcasts may only use unconfirmed user data
        if broadcast.is_some() {
            return match header.control.func {
                Function::PriUnconfirmedUserData => (
                    Some(FrameInfo::new(source, broadcast, FrameType::Data)),
                    None,
                ),
                _ => {
                    tracing::warn!(
                        "ignoring broadcast frame with function: {:?}",
                        header.control.func
                    );
                    (None, None)
                }
            };
        }

        match header.control.func {
            Function::PriUnconfirmedUserData => {
                if header.control.fcv {
                    tracing::warn!("ignoring frame of unconfirmed user data with FCV bit set");
                    return (None, None);
                }

                (
                    Some(FrameInfo::new(source, broadcast, FrameType::Data)),
                    None,
                )
            }
            Function::PriResetLinkStates => {
                if header.control.fcv {
                    tracing::warn!("ignoring reset link states with FCV bit set");
                    return (None, None);
                }

                self.secondary_state = SecondaryState::Reset(true);
                (None, Some(Reply::new(source, Function::SecAck)))
            }
            Function::PriConfirmedUserData => {
                if !header.control.fcv {
                    tracing::warn!("ignoring frame of confirmed user data with FCV bit unset");
                    return (None, None);
                }

                match self.secondary_state {
                    SecondaryState::NotReset => {
                        tracing::info!(
                            "ignoring confirmed user data while secondary state is not reset"
                        );
                        (None, None)
                    }
                    SecondaryState::Reset(expected) => {
                        if header.control.fcb == expected {
                            self.secondary_state = SecondaryState::Reset(!expected);
                            (
                                Some(FrameInfo::new(source, broadcast, FrameType::Data)),
                                Some(Reply::new(source, Function::SecAck)),
                            )
                        } else {
                            tracing::info!("ignoring confirmed user data with non-matching fcb");
                            (None, Some(Reply::new(source, Function::SecAck)))
                        }
                    }
                }
            }
            Function::PriRequestLinkStatus => {
                if header.control.fcv {
                    tracing::warn!("ignoring request link status with FCV bit set");
                    return (None, None);
                }

                (
                    Some(FrameInfo::new(
                        source,
                        broadcast,
                        FrameType::LinkStatusRequest,
                    )),
                    Some(Reply::new(source, Function::SecLinkStatus)),
                )
            }
            Function::SecLinkStatus => (
                Some(FrameInfo::new(
                    source,
                    broadcast,
                    FrameType::LinkStatusResponse,
                )),
                None,
            ),
            function => {
                tracing::warn!("ignoring frame with function code: {:?}", function);
                (None, None)
            }
        }
    }
}
