use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::function::Function;
use crate::link::header::{
    AnyAddress, BroadcastConfirmMode, ControlField, FrameInfo, FrameType, Header,
};
use crate::link::parser::FramePayload;
use crate::tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::link::format::format_header;
use crate::outstation::config::SelfAddressSupport;

enum SecondaryState {
    NotReset,
    Reset(bool), // the next expected fcb
}

pub(crate) struct Layer {
    endpoint_type: EndpointType,
    self_address_support: SelfAddressSupport,
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
        endpoint_type: EndpointType,
        self_address_support: SelfAddressSupport,
        local_address: EndpointAddress,
    ) -> Self {
        Self {
            endpoint_type,
            self_address_support,
            local_address,
            secondary_state: SecondaryState::NotReset,
            reader: super::reader::Reader::default(),
            tx_buffer: [0; super::constant::LINK_HEADER_LENGTH],
        }
    }

    pub(crate) fn reset(&mut self) {
        self.secondary_state = SecondaryState::NotReset;
        self.reader.reset();
    }

    pub(crate) async fn read<T>(
        &mut self,
        io: &mut T,
        payload: &mut FramePayload,
    ) -> Result<FrameInfo, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            if let Some(address) = self.read_one(io, payload).await? {
                return Ok(address);
            }
        }
    }

    fn format_reply(&mut self, reply: Reply) -> &[u8] {
        format_header(
            ControlField::new(self.endpoint_type.dir_bit(), reply.function),
            reply.address.raw_value(),
            self.local_address.raw_value(),
            &mut self.tx_buffer,
        );
        &self.tx_buffer
    }

    async fn read_one<T>(
        &mut self,
        io: &mut T,
        payload: &mut FramePayload,
    ) -> Result<Option<FrameInfo>, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let header = self.reader.read(io, payload).await?;
        let (info, reply) = self.process_header(&header);
        if let Some(reply) = reply {
            io.write_all(self.format_reply(reply)).await?
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
                log::warn!(
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
                    log::warn!("ignoring frame sent to address: {}", x);
                    return (None, None);
                }
            }
            AnyAddress::SelfAddress => {
                match self.self_address_support {
                    SelfAddressSupport::Enabled => None, // just pretend like it was sent to us
                    SelfAddressSupport::Disabled => {
                        log::warn!("ignoring frame sent to self address");
                        return (None, None);
                    }
                }
            }
            AnyAddress::Reserved(x) => {
                log::warn!("ignoring frame sent to reserved address: {}", x);
                return (None, None);
            }
            AnyAddress::Broadcast(mode) => match self.endpoint_type {
                EndpointType::Master => {
                    log::warn!("ignoring broadcast frame sent to master");
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
                    log::warn!(
                        "ignoring broadcast frame with function: {:?}",
                        header.control.func
                    );
                    (None, None)
                }
            };
        }

        match header.control.func {
            Function::PriUnconfirmedUserData => (
                Some(FrameInfo::new(source, broadcast, FrameType::Data)),
                None,
            ),
            Function::PriResetLinkStates => {
                self.secondary_state = SecondaryState::Reset(true); // TODO - does it start true or false
                (None, Some(Reply::new(source, Function::SecAck)))
            }
            Function::PriConfirmedUserData => match self.secondary_state {
                SecondaryState::NotReset => {
                    log::info!("ignoring confirmed user data while secondary state is not reset");
                    (None, None)
                }
                SecondaryState::Reset(expected) => {
                    if header.control.fcb == expected {
                        self.secondary_state = SecondaryState::Reset(!expected);
                        (
                            Some(FrameInfo::new(source, broadcast, FrameType::Data)),
                            None,
                        )
                    } else {
                        log::info!("ignoring confirmed user data with non-matching fcb");
                        (None, None)
                    }
                }
            },
            Function::PriRequestLinkStatus => (
                Some(FrameInfo::new(
                    source,
                    broadcast,
                    FrameType::LinkStatusRequest,
                )),
                Some(Reply::new(source, Function::SecLinkStatus)),
            ),
            Function::SecLinkStatus => (
                Some(FrameInfo::new(
                    source,
                    broadcast,
                    FrameType::LinkStatusResponse,
                )),
                None,
            ),
            function => {
                log::warn!("ignoring frame with function code: {:?}", function);
                (None, None)
            }
        }
    }
}
