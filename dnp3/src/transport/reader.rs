use crate::entry::NormalAddress;
use crate::link::error::LinkError;
use crate::link::parser::FramePayload;
use crate::transport::assembler::{Assembler, AssemblyState};
use crate::transport::header::Header;
use crate::transport::{Fragment, TransportType};
use tokio::prelude::{AsyncRead, AsyncWrite};

pub(crate) struct Reader {
    link: crate::link::layer::Layer,
    assembler: Assembler,
}

impl Reader {
    pub(crate) fn new(tt: TransportType, address: NormalAddress) -> Self {
        Self {
            link: crate::link::layer::Layer::new(tt.is_master(), address),
            assembler: Assembler::new(),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.assembler.reset();
        self.link.reset();
    }

    pub(crate) fn peek(&self) -> Option<Fragment> {
        self.assembler.peek()
    }

    pub(crate) async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
    where
        T: AsyncRead + AsyncWrite + Unpin,
    {
        let mut payload = FramePayload::new();

        loop {
            let address = self.link.read(io, &mut payload).await?;
            match payload.get() {
                [transport, data @ ..] => {
                    let header = Header::new(*transport);
                    if let AssemblyState::Complete = self.assembler.assemble(address, header, data)
                    {
                        return Ok(());
                    }
                }
                [] => log::warn!("received link data frame with no payload"),
            }
        }
    }
}
