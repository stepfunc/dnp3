use crate::entry::NormalAddress;
use crate::link::error::LinkError;
use crate::link::formatter::LinkFormatter;
use crate::link::function::Function;
use crate::link::header::{Address, AddressPair, ControlField};
use crate::link::parser::FramePayload;
use crate::util::cursor::WriteCursor;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

enum SecondaryState {
    NotReset,
    Reset(bool), // the next expect fcb
}

pub(crate) struct Layer {
    local_address: NormalAddress,
    secondary_state: SecondaryState,
    formatter: LinkFormatter,
    reader: super::reader::Reader,
    tx_buffer: [u8; super::constant::MAX_LINK_FRAME_LENGTH],
}

impl Layer {
    pub(crate) fn new(is_master: bool, local_address: NormalAddress) -> Self {
        Self {
            local_address,
            secondary_state: SecondaryState::NotReset,
            formatter: LinkFormatter::new(is_master),
            reader: super::reader::Reader::default(),
            tx_buffer: [0; super::constant::MAX_LINK_FRAME_LENGTH],
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
    ) -> Result<AddressPair, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        loop {
            if let Some(address) = self.read_one(io, payload).await? {
                return Ok(address);
            }
        }
    }

    async fn reply<T>(
        &mut self,
        destination: Address,
        control: ControlField,
        io: &mut T,
    ) -> Result<(), LinkError>
    where
        T: AsyncWrite + Unpin,
    {
        let mut cursor = WriteCursor::new(self.tx_buffer.as_mut());
        let start = cursor.position();
        let addresses = AddressPair::new(destination, self.local_address.wrap());
        self.formatter
            .format_header_only(addresses, control, &mut cursor)?;
        let reply_frame = cursor.written_since(start)?;
        Ok(io.write_all(reply_frame).await?)
    }

    async fn acknowledge<T>(&mut self, destination: Address, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncWrite + Unpin,
    {
        self.reply(
            destination,
            ControlField::new(self.formatter.is_master(), Function::SecAck),
            io,
        )
        .await
    }

    async fn read_one<T>(
        &mut self,
        io: &mut T,
        payload: &mut FramePayload,
    ) -> Result<Option<AddressPair>, LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let header = self.reader.read(io, payload).await?;

        if header.control.master == self.formatter.is_master() {
            // TODO - more useful on TCP than multi-drop serial where
            // you have to ignore frames
            if header.control.master {
                log::info!("ignoring link frame from master");
            } else {
                log::info!("ignoring link frame from outstation");
            }
            return Ok(None);
        }

        // TODO - handle broadcast
        if header.addresses.destination != self.local_address.wrap() {
            log::info!(
                "ignoring frame for destination address: {}",
                header.addresses.destination.value()
            );
            return Ok(None);
        }

        match header.control.func {
            Function::PriUnconfirmedUserData => Ok(Some(header.addresses)),
            Function::PriResetLinkStates => {
                self.secondary_state = SecondaryState::Reset(true); // TODO - does it start true or false?
                self.acknowledge(header.addresses.source, io).await?;
                Ok(None)
            }
            Function::PriConfirmedUserData => match self.secondary_state {
                SecondaryState::NotReset => {
                    log::info!("ignoring confirmed user data while secondary state is not reset");
                    Ok(None)
                }
                SecondaryState::Reset(expected) => {
                    if header.control.fcb == expected {
                        self.secondary_state = SecondaryState::Reset(!expected);
                        Ok(Some(header.addresses))
                    } else {
                        log::info!("ignoring confirmed user data with non-matching fcb");
                        Ok(None)
                    }
                }
            },
            Function::PriRequestLinkStatus => {
                self.reply(
                    header.addresses.source,
                    ControlField::new(self.formatter.is_master(), Function::SecLinkStatus),
                    io,
                )
                .await?;
                Ok(None)
            }
            function => {
                log::info!("ignoring frame with function code: {:?}", function);
                Ok(None)
            }
        }
    }
}
