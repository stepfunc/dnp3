use crate::app::EndpointType;
use crate::entry::EndpointAddress;
use crate::link::error::LinkError;
use crate::link::parser::FramePayload;
use crate::outstation::config::Feature;
use crate::tokio::io::{AsyncRead, AsyncWrite};
use crate::transport::real::assembler::{Assembler, AssemblyState};
use crate::transport::real::header::Header;
use crate::transport::Fragment;

pub(crate) struct Reader {
    link: crate::link::layer::Layer,
    assembler: Assembler,
}

impl Reader {
    pub(crate) fn master(source: EndpointAddress, max_tx_buffer: usize) -> Self {
        Self {
            link: crate::link::layer::Layer::new(EndpointType::Master, Feature::Disabled, source),
            assembler: Assembler::new(max_tx_buffer),
        }
    }

    pub(crate) fn outstation(
        source: EndpointAddress,
        self_address: Feature,
        max_rx_buffer: usize,
    ) -> Self {
        Self {
            link: crate::link::layer::Layer::new(EndpointType::Outstation, self_address, source),
            assembler: Assembler::new(max_rx_buffer),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.assembler.reset();
        self.link.reset();
    }

    pub(crate) fn pop(&mut self) -> Option<Fragment> {
        self.assembler.pop()
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        self.assembler.peek()
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        if self.assembler.peek().is_some() {
            return Ok(());
        }

        let mut payload = FramePayload::new();

        loop {
            let info = self.link.read(io, &mut payload).await?;
            match payload.get() {
                [transport, data @ ..] => {
                    let header = Header::new(*transport);
                    if let AssemblyState::Complete = self.assembler.assemble(info, header, data) {
                        return Ok(());
                    }
                }
                [] => log::warn!("received link data frame with no payload"),
            }
        }
    }
}
