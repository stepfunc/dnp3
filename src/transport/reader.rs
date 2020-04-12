use crate::link::error::LinkError;
use crate::link::parser::FramePayload;
use crate::transport::assembler::{Assembler, AssemblyState, Fragment};
use crate::transport::header::Header;
use tokio::prelude::{AsyncRead, AsyncWrite};

pub struct Reader {
    link: crate::link::layer::Layer,
    assembler: Assembler,
}

impl Reader {
    pub fn new(is_master: bool, address: u16) -> Self {
        Self {
            link: crate::link::layer::Layer::new(is_master, address),
            assembler: Assembler::new(),
        }
    }

    pub fn reset(&mut self) {
        self.assembler.reset();
        self.link.reset();
    }

    pub fn peek(&self) -> Option<Fragment> {
        self.assembler.peek()
    }

    pub async fn read<T>(&mut self, io: &mut T) -> Result<(), LinkError>
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
